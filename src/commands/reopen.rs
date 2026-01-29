//! brd reopen command.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::issue::Status;
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues, resolve_issue_id};

pub fn cmd_reopen(cli: &Cli, paths: &RepoPaths, id: &str) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(id, &issues)?;

    {
        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(id.to_string()))?;

        issue.frontmatter.status = Status::Open;
        issue.frontmatter.owner = None;
        issue.frontmatter.completed_at = None;

        let issue_path = paths.issues_dir(&config).join(format!("{}.md", full_id));
        issue.save(&issue_path)?;
    }

    if cli.json {
        let issue = issues.get(&full_id).unwrap();
        let json = issue_to_json(issue, &issues);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Reopened: {}", full_id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{TestRepo, test_cli};

    #[test]
    fn test_reopen_sets_status_and_clears_owner() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-aaaa")
            .status(Status::Done)
            .owner("tester")
            .create();

        cmd_reopen(&test_cli(), &repo.paths, "brd-aaaa").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues.get("brd-aaaa").unwrap();
        assert_eq!(issue.status(), Status::Open);
        assert!(issue.frontmatter.owner.is_none());
        assert!(issue.frontmatter.completed_at.is_none());
    }

    #[test]
    fn test_reopen_from_skip() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-aaaa").status(Status::Skip).create();

        cmd_reopen(&test_cli(), &repo.paths, "brd-aaaa").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues.get("brd-aaaa").unwrap();
        assert_eq!(issue.status(), Status::Open);
    }

    #[test]
    fn test_reopen_issue_not_found() {
        let repo = TestRepo::builder().build();
        let err = cmd_reopen(&test_cli(), &repo.paths, "brd-missing").unwrap_err();
        assert!(matches!(err, BrdError::IssueNotFound(_)));
    }

    #[test]
    fn test_reopen_ambiguous_id() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-aaaa").create();
        repo.issue("brd-aaab").create();

        let err = cmd_reopen(&test_cli(), &repo.paths, "aaa").unwrap_err();
        assert!(matches!(err, BrdError::AmbiguousId(_, _)));
    }
}
