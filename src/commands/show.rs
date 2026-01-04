//! brd show command.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::graph::{compute_derived, get_dependents};
use crate::issue::Issue;
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues, resolve_issue_id};

fn format_show_output(issue: &Issue, issues: &HashMap<String, Issue>, json: bool) -> String {
    if json {
        let mut output = serde_json::to_string_pretty(&issue_to_json(issue, issues)).unwrap();
        output.push('\n');
        return output;
    }

    let mut output = String::new();

    let _ = writeln!(output, "ID:       {}", issue.id());
    let _ = writeln!(output, "Title:    {}", issue.title());
    let _ = writeln!(output, "Priority: {}", issue.priority());
    let _ = writeln!(output, "Status:   {}", issue.status());

    if !issue.deps().is_empty() {
        let _ = writeln!(output, "Deps:     {}", issue.deps().join(", "));
    }

    let dependents = get_dependents(issue.id(), issues);
    if !dependents.is_empty() {
        let _ = writeln!(output, "Dependents: {}", dependents.join(", "));
    }

    if !issue.tags().is_empty() {
        let _ = writeln!(output, "Tags:     {}", issue.tags().join(", "));
    }

    if let Some(owner) = &issue.frontmatter.owner {
        let _ = writeln!(output, "Owner:    {}", owner);
    }

    let derived = compute_derived(issue, issues);
    if derived.is_ready {
        let _ = writeln!(output, "State:    READY");
    } else if derived.is_blocked {
        let _ = writeln!(output, "State:    BLOCKED");
        if !derived.open_deps.is_empty() {
            let _ = writeln!(output, "  open:   {}", derived.open_deps.join(", "));
        }
        if !derived.missing_deps.is_empty() {
            let _ = writeln!(output, "  missing: {}", derived.missing_deps.join(", "));
        }
    }

    if !issue.frontmatter.acceptance.is_empty() {
        let _ = writeln!(output);
        let _ = writeln!(output, "Acceptance:");
        for ac in &issue.frontmatter.acceptance {
            let _ = writeln!(output, "  - {}", ac);
        }
    }

    if !issue.body.is_empty() {
        let _ = writeln!(output);
        let _ = writeln!(output, "{}", issue.body);
    }

    output
}

pub fn cmd_show(cli: &Cli, paths: &RepoPaths, id: &str, context: bool) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(id, &issues)?;
    let issue = issues
        .get(&full_id)
        .ok_or_else(|| BrdError::IssueNotFound(id.to_string()))?;

    if context && !cli.json {
        let output = format_context_output(issue, &issues);
        print!("{output}");
    } else {
        let output = format_show_output(issue, &issues, cli.json);
        print!("{output}");
    }

    Ok(())
}

/// format output with full context: the issue plus all deps and dependents content.
fn format_context_output(issue: &Issue, issues: &HashMap<String, Issue>) -> String {
    let mut output = String::new();

    // main issue
    let _ = writeln!(output, "=== {}: {} ===", issue.id(), issue.title());
    let _ = writeln!(output);
    let _ = write!(output, "{}", format_show_output(issue, issues, false));

    // dependencies
    let deps = issue.deps();
    if !deps.is_empty() {
        let _ = writeln!(output);
        let _ = writeln!(output, "=== Dependencies ===");
        for dep_id in deps {
            if let Some(dep_issue) = issues.get(dep_id) {
                let _ = writeln!(output);
                let _ = writeln!(output, "--- {} ({}) ---", dep_id, dep_issue.status());
                let _ = writeln!(output);
                if !dep_issue.body.is_empty() {
                    let _ = writeln!(output, "{}", dep_issue.body);
                } else {
                    let _ = writeln!(output, "(no description)");
                }
            } else {
                let _ = writeln!(output);
                let _ = writeln!(output, "--- {} (missing) ---", dep_id);
            }
        }
    }

    // dependents
    let dependents = get_dependents(issue.id(), issues);
    if !dependents.is_empty() {
        let _ = writeln!(output);
        let _ = writeln!(output, "=== Dependents ===");
        for dep_id in &dependents {
            if let Some(dep_issue) = issues.get(dep_id) {
                let _ = writeln!(output);
                let _ = writeln!(output, "--- {} ({}) ---", dep_id, dep_issue.status());
                let _ = writeln!(output);
                if !dep_issue.body.is_empty() {
                    let _ = writeln!(output, "{}", dep_issue.body);
                } else {
                    let _ = writeln!(output, "(no description)");
                }
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::{IssueType, Priority, Status};
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
    fn test_format_show_output_text_includes_sections() {
        let mut issue = Issue::new(
            "brd-1234".to_string(),
            "test issue".to_string(),
            Priority::P1,
            vec!["brd-aaaa".to_string(), "brd-missing".to_string()],
        );
        issue.frontmatter.tags = vec!["visual".to_string(), "urgent".to_string()];
        issue.frontmatter.owner = Some("agent-one".to_string());
        issue.frontmatter.acceptance = vec!["do a thing".to_string(), "do another".to_string()];
        issue.frontmatter.issue_type = Some(IssueType::Meta);
        issue.body = "more details".to_string();

        let mut dep_issue = Issue::new(
            "brd-aaaa".to_string(),
            "dep issue".to_string(),
            Priority::P2,
            vec![],
        );
        dep_issue.frontmatter.status = Status::Todo;

        let mut issues = HashMap::new();
        issues.insert(issue.id().to_string(), issue.clone());
        issues.insert(dep_issue.id().to_string(), dep_issue);

        let output = format_show_output(&issue, &issues, false);

        assert!(output.contains("ID:       brd-1234"));
        assert!(output.contains("Title:    test issue"));
        assert!(output.contains("Priority: P1"));
        assert!(output.contains("Status:   todo"));
        assert!(output.contains("Deps:     brd-aaaa, brd-missing"));
        assert!(output.contains("Tags:     visual, urgent"));
        assert!(output.contains("Owner:    agent-one"));
        assert!(output.contains("State:    BLOCKED"));
        assert!(output.contains("  open:   brd-aaaa"));
        assert!(output.contains("  missing: brd-missing"));
        assert!(output.contains("Acceptance:"));
        assert!(output.contains("  - do a thing"));
        assert!(output.contains("more details"));
    }

    #[test]
    fn test_format_show_output_json_includes_fields() {
        let mut issue = Issue::new(
            "brd-1234".to_string(),
            "test issue".to_string(),
            Priority::P2,
            vec!["brd-aaaa".to_string()],
        );
        issue.frontmatter.tags = vec!["visual".to_string()];

        let mut dep_issue = Issue::new(
            "brd-aaaa".to_string(),
            "dep issue".to_string(),
            Priority::P3,
            vec![],
        );
        dep_issue.frontmatter.status = Status::Done;

        let mut issues = HashMap::new();
        issues.insert(issue.id().to_string(), issue.clone());
        issues.insert(dep_issue.id().to_string(), dep_issue);

        let output = format_show_output(&issue, &issues, true);
        let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

        assert_eq!(json["id"], "brd-1234");
        assert_eq!(json["title"], "test issue");
        assert_eq!(json["priority"], "P2");
        assert_eq!(json["tags"], serde_json::json!(["visual"]));
        assert_eq!(json["derived"]["is_ready"], true);
    }

    #[test]
    fn test_cmd_show_ambiguous_id() {
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
        let err = cmd_show(&cli, &paths, "aaa", false).unwrap_err();
        assert!(matches!(err, BrdError::AmbiguousId(_, _)));
    }

    #[test]
    fn test_cmd_show_issue_not_found() {
        let (_dir, paths, _config) = create_test_repo();

        let cli = make_cli(false);
        let err = cmd_show(&cli, &paths, "brd-missing", false).unwrap_err();
        assert!(matches!(err, BrdError::IssueNotFound(_)));
    }
}
