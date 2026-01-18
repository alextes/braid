//! brd set command.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::issue::{IssueType, Priority, Status};
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
            "status" | "s" => {
                let status: Status = value.parse()?;
                issue.frontmatter.status = status;
            }
            "type" | "t" => {
                if value == "-" {
                    issue.frontmatter.issue_type = None;
                } else {
                    let issue_type: IssueType = value.parse()?;
                    issue.frontmatter.issue_type = Some(issue_type);
                }
            }
            "owner" | "o" => {
                if value == "-" {
                    issue.frontmatter.owner = None;
                } else {
                    issue.frontmatter.owner = Some(value.to_string());
                }
            }
            "title" => {
                issue.frontmatter.title = value.to_string();
            }
            "tag" => {
                if let Some(tag) = value.strip_prefix('-') {
                    issue.frontmatter.tags.retain(|t| t != tag);
                } else {
                    let tag = value.strip_prefix('+').unwrap_or(value);
                    if !issue.frontmatter.tags.contains(&tag.to_string()) {
                        issue.frontmatter.tags.push(tag.to_string());
                    }
                }
            }
            _ => {
                return Err(BrdError::Other(format!(
                    "unknown field '{}'. supported: priority, status, type, owner, title, tag",
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
        // format value appropriately for each field type
        let display_value = match field.to_lowercase().as_str() {
            "priority" | "p" | "status" | "s" | "type" | "t" => value.to_uppercase(),
            _ => value.to_string(),
        };
        println!("{} {} = {}", full_id, field, display_value);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::{Issue, IssueType, Status};
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

    #[test]
    fn test_set_status() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-stat", Priority::P2);

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-stat", "status", "done").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-stat").unwrap();
        assert_eq!(issue.status(), Status::Done);
    }

    #[test]
    fn test_set_status_short() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-stat2", Priority::P2);

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-stat2", "s", "doing").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-stat2").unwrap();
        assert_eq!(issue.status(), Status::Doing);
    }

    #[test]
    fn test_set_type() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-type", Priority::P2);

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-type", "type", "design").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-type").unwrap();
        assert_eq!(issue.issue_type(), Some(IssueType::Design));
    }

    #[test]
    fn test_set_type_clear() {
        let (_dir, paths, config) = create_test_repo();
        let mut issue = Issue::new(
            "brd-tclear".to_string(),
            "test".to_string(),
            Priority::P2,
            vec![],
        );
        issue.frontmatter.issue_type = Some(IssueType::Design);
        issue.frontmatter.status = Status::Open;
        let issue_path = paths.issues_dir(&config).join("brd-tclear.md");
        issue.save(&issue_path).unwrap();

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-tclear", "t", "-").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-tclear").unwrap();
        assert_eq!(issue.issue_type(), None);
    }

    #[test]
    fn test_set_owner() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-owner", Priority::P2);

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-owner", "owner", "alice").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-owner").unwrap();
        assert_eq!(issue.frontmatter.owner, Some("alice".to_string()));
    }

    #[test]
    fn test_set_owner_clear() {
        let (_dir, paths, config) = create_test_repo();
        let mut issue = Issue::new(
            "brd-oclear".to_string(),
            "test".to_string(),
            Priority::P2,
            vec![],
        );
        issue.frontmatter.owner = Some("bob".to_string());
        issue.frontmatter.status = Status::Open;
        let issue_path = paths.issues_dir(&config).join("brd-oclear.md");
        issue.save(&issue_path).unwrap();

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-oclear", "o", "-").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-oclear").unwrap();
        assert_eq!(issue.frontmatter.owner, None);
    }

    #[test]
    fn test_set_title() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-title", Priority::P2);

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-title", "title", "new title here").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-title").unwrap();
        assert_eq!(issue.title(), "new title here");
    }

    #[test]
    fn test_set_tag_add() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-tag1", Priority::P2);

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-tag1", "tag", "+urgent").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-tag1").unwrap();
        assert!(issue.tags().contains(&"urgent".to_string()));
    }

    #[test]
    fn test_set_tag_add_bare() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(&paths, &config, "brd-tag2", Priority::P2);

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-tag2", "tag", "bug").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-tag2").unwrap();
        assert!(issue.tags().contains(&"bug".to_string()));
    }

    #[test]
    fn test_set_tag_remove() {
        let (_dir, paths, config) = create_test_repo();
        let mut issue = Issue::new(
            "brd-tag3".to_string(),
            "test".to_string(),
            Priority::P2,
            vec![],
        );
        issue.frontmatter.tags = vec!["bug".to_string(), "urgent".to_string()];
        issue.frontmatter.status = Status::Open;
        let issue_path = paths.issues_dir(&config).join("brd-tag3.md");
        issue.save(&issue_path).unwrap();

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-tag3", "tag", "-bug").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-tag3").unwrap();
        assert!(!issue.tags().contains(&"bug".to_string()));
        assert!(issue.tags().contains(&"urgent".to_string()));
    }

    #[test]
    fn test_set_tag_no_duplicate() {
        let (_dir, paths, config) = create_test_repo();
        let mut issue = Issue::new(
            "brd-tag4".to_string(),
            "test".to_string(),
            Priority::P2,
            vec![],
        );
        issue.frontmatter.tags = vec!["bug".to_string()];
        issue.frontmatter.status = Status::Open;
        let issue_path = paths.issues_dir(&config).join("brd-tag4.md");
        issue.save(&issue_path).unwrap();

        let cli = make_cli();
        cmd_set(&cli, &paths, "brd-tag4", "tag", "+bug").unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-tag4").unwrap();
        assert_eq!(issue.tags().len(), 1);
    }
}
