//! brd set command.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::issue::Priority;
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues, resolve_issue_id};

pub fn cmd_set(cli: &Cli, paths: &RepoPaths, id: &str, field: &str, value: &str) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(id, &issues)?;

    {
        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(id.to_string()))?;

        match field.to_lowercase().as_str() {
            "priority" | "p" => {
                let priority: Priority = value.parse()?;
                issue.frontmatter.priority = priority;
            }
            _ => {
                return Err(BrdError::Other(format!(
                    "unknown field '{}'. supported fields: priority",
                    field
                )));
            }
        }

        issue.touch();
        let issue_path = paths.issues_dir(&config).join(format!("{}.md", full_id));
        issue.save(&issue_path)?;
    }

    if cli.json {
        let issue = issues.get(&full_id).unwrap();
        let json = issue_to_json(issue, &issues);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("{} {} = {}", full_id, field, value.to_uppercase());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::{Issue, Status};
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

    fn write_issue(paths: &RepoPaths, config: &Config, id: &str, priority: Priority) {
        let mut issue = Issue::new(id.to_string(), format!("issue {}", id), priority, vec![]);
        issue.frontmatter.status = Status::Open;
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
    fn test_set_priority() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-aaaa", Priority::P3);

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-aaaa", "priority", "P1").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-aaaa").unwrap();
        assert_eq!(issue.priority(), Priority::P1);
    }

    #[test]
    fn test_set_priority_short_field() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-bbbb", Priority::P2);

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-bbbb", "p", "P0").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-bbbb").unwrap();
        assert_eq!(issue.priority(), Priority::P0);
    }

    #[test]
    fn test_set_invalid_field() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-cccc", Priority::P2);

        let cli = make_cli();
        let err = cmd_set(&cli, &paths, "brd-cccc", "unknown", "value").unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn test_set_invalid_priority() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-dddd", Priority::P2);

        let cli = make_cli();
        let err = cmd_set(&cli, &paths, "brd-dddd", "priority", "P9").unwrap_err();
        assert!(err.to_string().contains("invalid priority"));
    }

    #[test]
    fn test_set_issue_not_found() {
        let (_dir, paths, _config) = create_test_repo();
        let cli = make_cli();
        let err = cmd_set(&cli, &paths, "brd-missing", "priority", "P1").unwrap_err();
        assert!(matches!(err, BrdError::IssueNotFound(_)));
    }
}
