//! brd rm command.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::issue::Status;
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::{load_all_issues, resolve_issue_id};

pub fn cmd_rm(cli: &Cli, paths: &RepoPaths, id: &str, force: bool) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(id, &issues)?;

    let issue = issues
        .get(&full_id)
        .ok_or_else(|| BrdError::IssueNotFound(id.to_string()))?;

    // safety: refuse to delete issues in progress unless forced
    if issue.status() == Status::Doing && !force {
        return Err(BrdError::Other(format!(
            "issue {} is in progress (use --force to delete anyway)",
            full_id
        )));
    }

    // delete issue file
    let issue_path = paths.issues_dir(&config).join(format!("{}.md", full_id));
    std::fs::remove_file(&issue_path)?;

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "deleted": full_id,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Deleted: {}", full_id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::{Issue, Priority, Status};
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

    fn write_issue(paths: &RepoPaths, config: &Config, issue: &Issue) {
        let issue_path = paths.issues_dir(config).join(format!("{}.md", issue.id()));
        issue.save(&issue_path).unwrap();
    }

    fn make_cli(json: bool) -> Cli {
        Cli {
            json,
            repo: None,
            no_color: true,
            verbose: false,
            command: crate::cli::Command::Doctor,
        }
    }

    #[test]
    fn test_rm_deletes_issue_file() {
        let (_dir, paths, config) = create_test_repo();
        let issue = Issue::new(
            "brd-aaaa".to_string(),
            "issue a".to_string(),
            Priority::P2,
            vec![],
        );
        write_issue(&paths, &config, &issue);
        let issue_path = paths.issues_dir(&config).join("brd-aaaa.md");
        assert!(issue_path.exists());

        let cli = make_cli(false);
        cmd_rm(&cli, &paths, "brd-aaaa", false).unwrap();

        assert!(!issue_path.exists());
    }

    #[test]
    fn test_rm_requires_force_for_doing_issue() {
        let (_dir, paths, config) = create_test_repo();
        let mut issue = Issue::new(
            "brd-aaaa".to_string(),
            "issue a".to_string(),
            Priority::P2,
            vec![],
        );
        issue.frontmatter.status = Status::Doing;
        write_issue(&paths, &config, &issue);
        let issue_path = paths.issues_dir(&config).join("brd-aaaa.md");

        let cli = make_cli(false);
        let err = cmd_rm(&cli, &paths, "brd-aaaa", false).unwrap_err();
        assert!(err.to_string().contains("in progress"));
        assert!(issue_path.exists());
    }

    #[test]
    fn test_rm_force_deletes_doing_issue() {
        let (_dir, paths, config) = create_test_repo();
        let mut issue = Issue::new(
            "brd-aaaa".to_string(),
            "issue a".to_string(),
            Priority::P2,
            vec![],
        );
        issue.frontmatter.status = Status::Doing;
        write_issue(&paths, &config, &issue);
        let issue_path = paths.issues_dir(&config).join("brd-aaaa.md");

        let cli = make_cli(false);
        cmd_rm(&cli, &paths, "brd-aaaa", true).unwrap();

        assert!(!issue_path.exists());
    }

    #[test]
    fn test_rm_ambiguous_id() {
        let (_dir, paths, config) = create_test_repo();
        let issue_a = Issue::new(
            "brd-aaaa".to_string(),
            "issue a".to_string(),
            Priority::P2,
            vec![],
        );
        let issue_b = Issue::new(
            "brd-aaab".to_string(),
            "issue b".to_string(),
            Priority::P2,
            vec![],
        );
        write_issue(&paths, &config, &issue_a);
        write_issue(&paths, &config, &issue_b);

        let cli = make_cli(false);
        let err = cmd_rm(&cli, &paths, "aaa", false).unwrap_err();
        assert!(matches!(err, BrdError::AmbiguousId(_, _)));
    }
}
