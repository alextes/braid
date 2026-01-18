//! brd skip command.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::issue::Status;
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues, resolve_issue_id};

pub fn cmd_skip(cli: &Cli, paths: &RepoPaths, id: &str) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(id, &issues)?;

    {
        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(id.to_string()))?;

        issue.frontmatter.status = Status::Skip;
        issue.frontmatter.owner = None;
        issue.mark_completed();

        let issue_path = paths.issues_dir(&config).join(format!("{}.md", full_id));
        issue.save(&issue_path)?;
    }

    if cli.json {
        let issue = issues.get(&full_id).unwrap();
        let json = issue_to_json(issue, &issues);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Skipped: {}", full_id);
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

    #[test]
    fn test_skip_sets_status_and_clears_owner() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-aaaa", Status::Doing, Some("tester"));

        let cli = make_cli();
        cmd_skip(&cli, &paths, "brd-aaaa").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-aaaa").unwrap();
        assert_eq!(issue.status(), Status::Skip);
        assert!(issue.frontmatter.owner.is_none());
    }

    #[test]
    fn test_skip_issue_not_found() {
        let (_dir, paths, _config) = create_test_repo();
        let cli = make_cli();
        let err = cmd_skip(&cli, &paths, "brd-missing").unwrap_err();
        assert!(matches!(err, BrdError::IssueNotFound(_)));
    }

    #[test]
    fn test_skip_ambiguous_id() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-aaaa", Status::Open, None);
        write_issue(&paths, &config, "brd-aaab", Status::Open, None);

        let cli = make_cli();
        let err = cmd_skip(&cli, &paths, "aaa").unwrap_err();
        assert!(matches!(err, BrdError::AmbiguousId(_, _)));
    }
}
