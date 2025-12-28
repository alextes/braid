//! brd done command.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::issue::{IssueType, Status};
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues, resolve_issue_id};

pub fn cmd_done(
    cli: &Cli,
    paths: &RepoPaths,
    id: &str,
    force: bool,
    result_ids: &[String],
) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(id, &issues)?;

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
    for result_id in result_ids {
        let resolved = resolve_issue_id(result_id, &issues)?;
        resolved_results.push(resolved);
    }

    // propagate deps: find issues that depend on this one and add result issues as their deps
    if !resolved_results.is_empty() {
        let dependents: Vec<String> = issues
            .iter()
            .filter(|(_, issue)| issue.deps().contains(&full_id))
            .map(|(id, _)| id.clone())
            .collect();

        for dependent_id in &dependents {
            if let Some(dependent) = issues.get_mut(dependent_id) {
                for result_id in &resolved_results {
                    if !dependent.frontmatter.deps.contains(result_id) {
                        dependent.frontmatter.deps.push(result_id.clone());
                    }
                }
                dependent.touch();
                let dep_path = paths
                    .issues_dir(&config)
                    .join(format!("{}.md", dependent_id));
                dependent.save(&dep_path)?;
            }
        }

        if !cli.json && !dependents.is_empty() {
            eprintln!(
                "propagated deps to {} issue(s): {}",
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
        issue.touch();

        let issue_path = paths.issues_dir(&config).join(format!("{}.md", full_id));
        issue.save(&issue_path)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_repo() -> (tempfile::TempDir, RepoPaths, Config) {
        let dir = tempdir().unwrap();
        let paths = RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        };
        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        fs::create_dir_all(paths.braid_dir().join("issues")).unwrap();
        let config = Config::default();
        config.save(&paths.config_path()).unwrap();
        (dir, paths, config)
    }

    fn write_issue(
        paths: &RepoPaths,
        config: &Config,
        id: &str,
        status: Status,
        owner: Option<&str>,
    ) {
        let mut issue = crate::issue::Issue::new(
            id.to_string(),
            format!("issue {}", id),
            crate::issue::Priority::P2,
            vec![],
        );
        issue.frontmatter.status = status;
        issue.frontmatter.owner = owner.map(|o| o.to_string());
        let issue_path = paths.issues_dir(config).join(format!("{}.md", id));
        issue.save(&issue_path).unwrap();
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

    fn write_design_issue(paths: &RepoPaths, config: &Config, id: &str) {
        let mut issue = crate::issue::Issue::new(
            id.to_string(),
            format!("design {}", id),
            crate::issue::Priority::P2,
            vec![],
        );
        issue.frontmatter.issue_type = Some(IssueType::Design);
        let issue_path = paths.issues_dir(config).join(format!("{}.md", id));
        issue.save(&issue_path).unwrap();
    }

    fn write_issue_with_deps(paths: &RepoPaths, config: &Config, id: &str, deps: Vec<String>) {
        let issue = crate::issue::Issue::new(
            id.to_string(),
            format!("issue {}", id),
            crate::issue::Priority::P2,
            deps,
        );
        let issue_path = paths.issues_dir(config).join(format!("{}.md", id));
        issue.save(&issue_path).unwrap();
    }

    #[test]
    fn test_done_sets_status_and_clears_owner() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-aaaa", Status::Doing, Some("tester"));

        let cli = make_cli();
        cmd_done(&cli, &paths, "brd-aaaa", false, &[]).unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-aaaa").unwrap();
        assert_eq!(issue.status(), Status::Done);
        assert!(issue.frontmatter.owner.is_none());
    }

    #[test]
    fn test_done_issue_not_found() {
        let (_dir, paths, _config) = create_test_repo();
        let cli = make_cli();
        let err = cmd_done(&cli, &paths, "brd-missing", false, &[]).unwrap_err();
        assert!(matches!(err, BrdError::IssueNotFound(_)));
    }

    #[test]
    fn test_done_ambiguous_id() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-aaaa", Status::Todo, None);
        write_issue(&paths, &config, "brd-aaab", Status::Todo, None);

        let cli = make_cli();
        let err = cmd_done(&cli, &paths, "aaa", false, &[]).unwrap_err();
        assert!(matches!(err, BrdError::AmbiguousId(_, _)));
    }

    #[test]
    fn test_done_design_requires_result() {
        let (_dir, paths, config) = create_test_repo();
        write_design_issue(&paths, &config, "brd-design");

        let cli = make_cli();
        let err = cmd_done(&cli, &paths, "brd-design", false, &[]).unwrap_err();
        assert!(err.to_string().contains("design issues require --result"));
    }

    #[test]
    fn test_done_design_with_force() {
        let (_dir, paths, config) = create_test_repo();
        write_design_issue(&paths, &config, "brd-design");

        let cli = make_cli();
        cmd_done(&cli, &paths, "brd-design", true, &[]).unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-design").unwrap();
        assert_eq!(issue.status(), Status::Done);
    }

    #[test]
    fn test_done_design_with_result() {
        let (_dir, paths, config) = create_test_repo();
        write_design_issue(&paths, &config, "brd-design");
        write_issue(&paths, &config, "brd-impl", Status::Todo, None);

        let cli = make_cli();
        cmd_done(&cli, &paths, "brd-design", false, &["brd-impl".to_string()]).unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-design").unwrap();
        assert_eq!(issue.status(), Status::Done);
    }

    #[test]
    fn test_done_design_result_not_found() {
        let (_dir, paths, config) = create_test_repo();
        write_design_issue(&paths, &config, "brd-design");

        let cli = make_cli();
        let err = cmd_done(
            &cli,
            &paths,
            "brd-design",
            false,
            &["brd-missing".to_string()],
        )
        .unwrap_err();
        assert!(matches!(err, BrdError::IssueNotFound(_)));
    }

    #[test]
    fn test_done_design_propagates_deps() {
        let (_dir, paths, config) = create_test_repo();
        write_design_issue(&paths, &config, "brd-design");
        write_issue(&paths, &config, "brd-impl", Status::Todo, None);
        write_issue_with_deps(
            &paths,
            &config,
            "brd-dependent",
            vec!["brd-design".to_string()],
        );

        let cli = make_cli();
        cmd_done(&cli, &paths, "brd-design", false, &["brd-impl".to_string()]).unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let dependent = issues.get("brd-dependent").unwrap();
        assert!(dependent.deps().contains(&"brd-impl".to_string()));
        assert!(dependent.deps().contains(&"brd-design".to_string()));
    }
}
