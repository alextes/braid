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
                // Update timestamps based on status transition
                if status == Status::Doing {
                    issue.mark_started();
                } else if matches!(status, Status::Done | Status::Skip) {
                    issue.mark_completed();
                }
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
    use crate::issue::{IssueType, Status};
    use crate::test_utils::{TestRepo, test_cli};

    #[test]
    fn test_set_priority() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-aaaa").priority(Priority::P3).create();

        cmd_set(&test_cli(), &repo.paths, "brd-aaaa", "priority", "P1").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.get("brd-aaaa").unwrap().priority(), Priority::P1);
    }

    #[test]
    fn test_set_priority_short_field() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-bbbb").priority(Priority::P2).create();

        cmd_set(&test_cli(), &repo.paths, "brd-bbbb", "p", "P0").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.get("brd-bbbb").unwrap().priority(), Priority::P0);
    }

    #[test]
    fn test_set_invalid_field() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-cccc").create();

        let err = cmd_set(&test_cli(), &repo.paths, "brd-cccc", "unknown", "value").unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn test_set_invalid_priority() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-dddd").create();

        let err = cmd_set(&test_cli(), &repo.paths, "brd-dddd", "priority", "P9").unwrap_err();
        assert!(err.to_string().contains("invalid priority"));
    }

    #[test]
    fn test_set_issue_not_found() {
        let repo = TestRepo::builder().build();

        let err = cmd_set(&test_cli(), &repo.paths, "brd-missing", "priority", "P1").unwrap_err();
        assert!(matches!(err, BrdError::IssueNotFound(_)));
    }

    #[test]
    fn test_set_status() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-stat").create();

        cmd_set(&test_cli(), &repo.paths, "brd-stat", "status", "done").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.get("brd-stat").unwrap().status(), Status::Done);
    }

    #[test]
    fn test_set_status_short() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-stat2").create();

        cmd_set(&test_cli(), &repo.paths, "brd-stat2", "s", "doing").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.get("brd-stat2").unwrap().status(), Status::Doing);
    }

    #[test]
    fn test_set_type() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-type").create();

        cmd_set(&test_cli(), &repo.paths, "brd-type", "type", "design").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(
            issues.get("brd-type").unwrap().issue_type(),
            Some(IssueType::Design)
        );
    }

    #[test]
    fn test_set_type_clear() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-tclear")
            .issue_type(IssueType::Design)
            .create();

        cmd_set(&test_cli(), &repo.paths, "brd-tclear", "t", "-").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.get("brd-tclear").unwrap().issue_type(), None);
    }

    #[test]
    fn test_set_owner() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-owner").create();

        cmd_set(&test_cli(), &repo.paths, "brd-owner", "owner", "alice").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(
            issues.get("brd-owner").unwrap().frontmatter.owner,
            Some("alice".to_string())
        );
    }

    #[test]
    fn test_set_owner_clear() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-oclear").owner("bob").create();

        cmd_set(&test_cli(), &repo.paths, "brd-oclear", "o", "-").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.get("brd-oclear").unwrap().frontmatter.owner, None);
    }

    #[test]
    fn test_set_title() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-title").create();

        cmd_set(
            &test_cli(),
            &repo.paths,
            "brd-title",
            "title",
            "new title here",
        )
        .unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.get("brd-title").unwrap().title(), "new title here");
    }

    #[test]
    fn test_set_tag_add() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-tag1").create();

        cmd_set(&test_cli(), &repo.paths, "brd-tag1", "tag", "+urgent").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert!(
            issues
                .get("brd-tag1")
                .unwrap()
                .tags()
                .contains(&"urgent".to_string())
        );
    }

    #[test]
    fn test_set_tag_add_bare() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-tag2").create();

        cmd_set(&test_cli(), &repo.paths, "brd-tag2", "tag", "bug").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert!(
            issues
                .get("brd-tag2")
                .unwrap()
                .tags()
                .contains(&"bug".to_string())
        );
    }

    #[test]
    fn test_set_tag_remove() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-tag3").tags(&["bug", "urgent"]).create();

        cmd_set(&test_cli(), &repo.paths, "brd-tag3", "tag", "-bug").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues.get("brd-tag3").unwrap();
        assert!(!issue.tags().contains(&"bug".to_string()));
        assert!(issue.tags().contains(&"urgent".to_string()));
    }

    #[test]
    fn test_set_tag_no_duplicate() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-tag4").tags(&["bug"]).create();

        cmd_set(&test_cli(), &repo.paths, "brd-tag4", "tag", "+bug").unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.get("brd-tag4").unwrap().tags().len(), 1);
    }
}
