//! brd add command.

use crate::cli::{AddArgs, Cli};
use crate::config::Config;
use crate::date::parse_scheduled_date;
use crate::error::Result;
use crate::issue::{Issue, IssueType, Priority};
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::{generate_issue_id, issue_to_json, load_all_issues, resolve_issue_id};

pub fn cmd_add(cli: &Cli, paths: &RepoPaths, args: &AddArgs) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let priority: Priority = args.priority.parse()?;
    let issue_type: Option<IssueType> = args.r#type.as_deref().map(|s| s.parse()).transpose()?;

    // resolve deps to full IDs
    let all_issues = load_all_issues(paths, &config)?;
    let resolved_deps: Vec<String> = args
        .dep
        .iter()
        .map(|d| resolve_issue_id(d, &all_issues))
        .collect::<Result<Vec<_>>>()?;

    // generate ID
    let issues_dir = paths.issues_dir(&config);
    let id = generate_issue_id(&config, &issues_dir)?;

    // parse scheduled_for if provided
    let scheduled_for = args
        .scheduled_for
        .as_ref()
        .map(|s| parse_scheduled_date(s))
        .transpose()?;

    // create issue
    let mut issue = Issue::new(id.clone(), args.title.clone(), priority, resolved_deps);
    issue.frontmatter.issue_type = issue_type;
    issue.frontmatter.acceptance = args.ac.clone();
    issue.frontmatter.tags = args.tag.clone();
    issue.frontmatter.scheduled_for = scheduled_for;
    if let Some(ref b) = args.body {
        issue.body = b.clone();
    }

    // save with lock
    let _lock = LockGuard::acquire(&paths.lock_path())?;
    let issue_path = issues_dir.join(format!("{}.md", id));
    issue.save(&issue_path)?;

    if cli.json {
        let json = issue_to_json(&issue, &all_issues);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Created issue: {}", id);
        println!("  {}", issue_path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{TestRepo, test_cli};

    fn make_args(title: &str) -> AddArgs {
        AddArgs {
            title: title.to_string(),
            priority: "P2".to_string(),
            r#type: None,
            dep: vec![],
            ac: vec![],
            tag: vec![],
            body: None,
            scheduled_for: None,
        }
    }

    // =========================================================================
    // Basic add tests
    // =========================================================================

    #[test]
    fn test_add_minimal() {
        let repo = TestRepo::builder().build();
        let args = make_args("Test issue");

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.len(), 1);

        let issue = issues.values().next().unwrap();
        assert_eq!(issue.title(), "Test issue");
        assert_eq!(issue.priority(), Priority::P2);
    }

    #[test]
    fn test_add_with_all_options() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-dep1").create();

        let args = AddArgs {
            title: "Full issue".to_string(),
            priority: "P0".to_string(),
            r#type: Some("design".to_string()),
            dep: vec!["brd-dep1".to_string()],
            ac: vec!["criterion 1".to_string(), "criterion 2".to_string()],
            tag: vec!["testing".to_string(), "urgent".to_string()],
            body: Some("This is the body".to_string()),
            scheduled_for: None,
        };

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.len(), 2);

        let new_issue = issues.values().find(|i| i.title() == "Full issue").unwrap();
        assert_eq!(new_issue.priority(), Priority::P0);
        assert_eq!(new_issue.issue_type(), Some(IssueType::Design));
        assert!(new_issue.deps().contains(&"brd-dep1".to_string()));
        assert_eq!(new_issue.frontmatter.acceptance.len(), 2);
        assert!(new_issue.tags().contains(&"testing".to_string()));
        assert!(new_issue.tags().contains(&"urgent".to_string()));
        assert_eq!(new_issue.body, "This is the body");
    }

    // =========================================================================
    // Priority parsing tests
    // =========================================================================

    #[test]
    fn test_add_priority_p0() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("P0 issue");
        args.priority = "P0".to_string();

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.values().next().unwrap().priority(), Priority::P0);
    }

    #[test]
    fn test_add_priority_p3() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("P3 issue");
        args.priority = "P3".to_string();

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.values().next().unwrap().priority(), Priority::P3);
    }

    #[test]
    fn test_add_invalid_priority() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("Bad priority");
        args.priority = "P5".to_string();

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_err());
    }

    // =========================================================================
    // Type parsing tests
    // =========================================================================

    #[test]
    fn test_add_type_design() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("Design issue");
        args.r#type = Some("design".to_string());

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(
            issues.values().next().unwrap().issue_type(),
            Some(IssueType::Design)
        );
    }

    #[test]
    fn test_add_type_meta() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("Meta issue");
        args.r#type = Some("meta".to_string());

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(
            issues.values().next().unwrap().issue_type(),
            Some(IssueType::Meta)
        );
    }

    #[test]
    fn test_add_invalid_type() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("Bad type");
        args.r#type = Some("invalid".to_string());

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_err());
    }

    // =========================================================================
    // Dependency tests
    // =========================================================================

    #[test]
    fn test_add_with_dep() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-parent").create();

        let mut args = make_args("Child issue");
        args.dep = vec!["brd-parent".to_string()];

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let child = issues
            .values()
            .find(|i| i.title() == "Child issue")
            .unwrap();
        assert!(child.deps().contains(&"brd-parent".to_string()));
    }

    #[test]
    fn test_add_with_multiple_deps() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-dep1").create();
        repo.issue("brd-dep2").create();

        let mut args = make_args("Multi dep issue");
        args.dep = vec!["brd-dep1".to_string(), "brd-dep2".to_string()];

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues
            .values()
            .find(|i| i.title() == "Multi dep issue")
            .unwrap();
        assert!(issue.deps().contains(&"brd-dep1".to_string()));
        assert!(issue.deps().contains(&"brd-dep2".to_string()));
    }

    #[test]
    fn test_add_with_partial_dep_id() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-abcd").create();

        let mut args = make_args("Partial dep issue");
        args.dep = vec!["abcd".to_string()]; // partial ID

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues
            .values()
            .find(|i| i.title() == "Partial dep issue")
            .unwrap();
        assert!(issue.deps().contains(&"brd-abcd".to_string()));
    }

    #[test]
    fn test_add_with_nonexistent_dep() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("Bad dep issue");
        args.dep = vec!["nonexistent".to_string()];

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_err());
    }

    // =========================================================================
    // ID generation tests
    // =========================================================================

    #[test]
    fn test_add_generates_unique_ids() {
        let repo = TestRepo::builder().build();

        for i in 0..5 {
            let args = make_args(&format!("Issue {}", i));
            let result = cmd_add(&test_cli(), &repo.paths, &args);
            assert!(result.is_ok());
        }

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.len(), 5);

        let ids: std::collections::HashSet<_> = issues.keys().collect();
        assert_eq!(ids.len(), 5);
    }

    #[test]
    fn test_add_uses_config_prefix() {
        let repo = TestRepo::builder().build();

        // Override config with custom prefix
        let config = Config {
            id_prefix: "test".to_string(),
            ..Default::default()
        };
        config.save(&repo.paths.config_path()).unwrap();

        let args = make_args("Custom prefix issue");
        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &config).unwrap();
        let id = issues.keys().next().unwrap();
        assert!(id.starts_with("test-"));
    }

    // =========================================================================
    // Tags and acceptance criteria tests
    // =========================================================================

    #[test]
    fn test_add_with_tags() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("Tagged issue");
        args.tag = vec!["bug".to_string(), "frontend".to_string()];

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues.values().next().unwrap();
        assert!(issue.tags().contains(&"bug".to_string()));
        assert!(issue.tags().contains(&"frontend".to_string()));
    }

    #[test]
    fn test_add_with_acceptance_criteria() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("AC issue");
        args.ac = vec![
            "Users can log in".to_string(),
            "Error messages are shown".to_string(),
        ];

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues.values().next().unwrap();
        assert_eq!(issue.frontmatter.acceptance.len(), 2);
        assert!(
            issue
                .frontmatter
                .acceptance
                .contains(&"Users can log in".to_string())
        );
    }

    // =========================================================================
    // Body tests
    // =========================================================================

    #[test]
    fn test_add_with_body() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("Body issue");
        args.body = Some("This is the issue body\nwith multiple lines".to_string());

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues.values().next().unwrap();
        assert!(issue.body.contains("multiple lines"));
    }

    #[test]
    fn test_add_without_body() {
        let repo = TestRepo::builder().build();
        let args = make_args("No body issue");

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues.values().next().unwrap();
        assert!(issue.body.is_empty());
    }

    // =========================================================================
    // Scheduled for tests
    // =========================================================================

    #[test]
    fn test_add_with_scheduled_for_relative() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("Scheduled issue");
        args.scheduled_for = Some("+7d".to_string());

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues.values().next().unwrap();
        assert!(issue.frontmatter.scheduled_for.is_some());
    }

    #[test]
    fn test_add_with_scheduled_for_iso() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("Scheduled issue ISO");
        args.scheduled_for = Some("2099-12-31".to_string());

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues.values().next().unwrap();
        let scheduled = issue.frontmatter.scheduled_for.unwrap();
        assert_eq!(scheduled.year(), 2099);
        assert_eq!(scheduled.month() as u8, 12);
        assert_eq!(scheduled.day(), 31);
    }

    #[test]
    fn test_add_with_scheduled_for_tomorrow() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("Tomorrow issue");
        args.scheduled_for = Some("tomorrow".to_string());

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_ok());

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues.values().next().unwrap();
        assert!(issue.frontmatter.scheduled_for.is_some());
    }

    #[test]
    fn test_add_with_invalid_scheduled_for() {
        let repo = TestRepo::builder().build();
        let mut args = make_args("Bad scheduled");
        args.scheduled_for = Some("invalid-date".to_string());

        let result = cmd_add(&test_cli(), &repo.paths, &args);
        assert!(result.is_err());
    }
}
