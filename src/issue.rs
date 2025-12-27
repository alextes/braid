//! issue file parsing and writing.

use serde::{Deserialize, Serialize};
use std::path::Path;
use time::OffsetDateTime;

use crate::error::{BrdError, Result};

/// priority levels (P0 highest to P3 lowest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::P0 => write!(f, "P0"),
            Priority::P1 => write!(f, "P1"),
            Priority::P2 => write!(f, "P2"),
            Priority::P3 => write!(f, "P3"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = BrdError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "P0" => Ok(Priority::P0),
            "P1" => Ok(Priority::P1),
            "P2" => Ok(Priority::P2),
            "P3" => Ok(Priority::P3),
            _ => Err(BrdError::ParseError(
                "priority".to_string(),
                format!("invalid priority: {s}"),
            )),
        }
    }
}

/// issue status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Todo,
    Doing,
    Done,
    Skip,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Todo => write!(f, "todo"),
            Status::Doing => write!(f, "doing"),
            Status::Done => write!(f, "done"),
            Status::Skip => write!(f, "skip"),
        }
    }
}

impl std::str::FromStr for Status {
    type Err = BrdError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "todo" => Ok(Status::Todo),
            "doing" => Ok(Status::Doing),
            "done" => Ok(Status::Done),
            "skip" => Ok(Status::Skip),
            _ => Err(BrdError::ParseError(
                "status".to_string(),
                format!("invalid status: {s}"),
            )),
        }
    }
}

/// issue type for categorization (e.g. design docs, meta/epic issues).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueType {
    Design,
    Meta,
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueType::Design => write!(f, "design"),
            IssueType::Meta => write!(f, "meta"),
        }
    }
}

impl std::str::FromStr for IssueType {
    type Err = BrdError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "design" => Ok(IssueType::Design),
            "meta" => Ok(IssueType::Meta),
            _ => Err(BrdError::ParseError(
                "type".to_string(),
                format!("invalid type: {s} (valid: design, meta)"),
            )),
        }
    }
}

/// the frontmatter of an issue file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueFrontmatter {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub priority: Priority,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub issue_type: Option<IssueType>,
    #[serde(default)]
    pub deps: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance: Vec<String>,
}

/// a complete issue with frontmatter and markdown body.
#[derive(Debug, Clone)]
pub struct Issue {
    pub frontmatter: IssueFrontmatter,
    pub body: String,
}

impl Issue {
    /// create a new issue with the given parameters.
    pub fn new(id: String, title: String, priority: Priority, deps: Vec<String>) -> Self {
        use crate::migrate::CURRENT_SCHEMA;
        let now = OffsetDateTime::now_utc();
        Self {
            frontmatter: IssueFrontmatter {
                schema_version: CURRENT_SCHEMA,
                id,
                title,
                priority,
                status: Status::Todo,
                issue_type: None,
                deps,
                tags: Vec::new(),
                owner: None,
                created_at: now,
                updated_at: now,
                acceptance: Vec::new(),
            },
            body: String::new(),
        }
    }

    /// parse an issue from a markdown file with YAML frontmatter.
    pub fn parse(content: &str) -> Result<Self> {
        use crate::migrate::{CURRENT_SCHEMA, migrate_frontmatter};

        let (frontmatter_str, body) = split_frontmatter(content)?;

        // Parse as generic YAML first to check version and migrate if needed
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(frontmatter_str)
            .map_err(|e| BrdError::ParseError("issue frontmatter".to_string(), e.to_string()))?;

        // Migrate in-memory if needed
        let (migrated, _) = migrate_frontmatter(yaml_value, CURRENT_SCHEMA)?;

        // Now deserialize into strongly-typed struct
        let frontmatter: IssueFrontmatter = serde_yaml::from_value(migrated)
            .map_err(|e| BrdError::ParseError("issue frontmatter".to_string(), e.to_string()))?;

        Ok(Self {
            frontmatter,
            body: body.to_string(),
        })
    }

    /// load an issue from a file path.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let issue = Self::parse(&content)?;

        // validate id matches filename
        let expected_id = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
            BrdError::ParseError(path.display().to_string(), "invalid filename".to_string())
        })?;

        if issue.frontmatter.id != expected_id {
            return Err(BrdError::ParseError(
                path.display().to_string(),
                format!(
                    "id '{}' does not match filename '{}'",
                    issue.frontmatter.id, expected_id
                ),
            ));
        }

        Ok(issue)
    }

    /// serialize the issue to markdown with YAML frontmatter.
    pub fn to_markdown(&self) -> Result<String> {
        let frontmatter_str = serde_yaml::to_string(&self.frontmatter)
            .map_err(|e| BrdError::Other(format!("failed to serialize frontmatter: {e}")))?;

        let mut output = String::new();
        output.push_str("---\n");
        output.push_str(&frontmatter_str);
        output.push_str("---\n");
        if !self.body.is_empty() {
            output.push('\n');
            output.push_str(&self.body);
        }

        Ok(output)
    }

    /// save the issue to a file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = self.to_markdown()?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// update the updated_at timestamp to now.
    pub fn touch(&mut self) {
        self.frontmatter.updated_at = OffsetDateTime::now_utc();
    }

    /// convenience accessors
    pub fn id(&self) -> &str {
        &self.frontmatter.id
    }

    pub fn title(&self) -> &str {
        &self.frontmatter.title
    }

    pub fn priority(&self) -> Priority {
        self.frontmatter.priority
    }

    pub fn status(&self) -> Status {
        self.frontmatter.status
    }

    pub fn issue_type(&self) -> Option<IssueType> {
        self.frontmatter.issue_type
    }

    pub fn deps(&self) -> &[String] {
        &self.frontmatter.deps
    }

    pub fn tags(&self) -> &[String] {
        &self.frontmatter.tags
    }

    /// compare two issues by priority, then created_at, then id.
    /// this is the canonical sort order for issue listings.
    pub fn cmp_by_priority(&self, other: &Self) -> std::cmp::Ordering {
        self.priority()
            .cmp(&other.priority())
            .then_with(|| {
                self.frontmatter
                    .created_at
                    .cmp(&other.frontmatter.created_at)
            })
            .then_with(|| self.id().cmp(other.id()))
    }
}

use std::collections::HashMap;

use crate::config::Config;
use rand::Rng;

/// resolve a partial issue ID to a full ID.
pub fn resolve_issue_id(partial: &str, issues: &HashMap<String, Issue>) -> Result<String> {
    // exact match
    if issues.contains_key(partial) {
        return Ok(partial.to_string());
    }

    // partial match
    let matches: Vec<&str> = issues
        .keys()
        .filter(|id| id.contains(partial) || id.ends_with(partial))
        .map(|s| s.as_str())
        .collect();

    match matches.len() {
        0 => Err(BrdError::IssueNotFound(partial.to_string())),
        1 => Ok(matches[0].to_string()),
        _ => Err(BrdError::AmbiguousId(
            partial.to_string(),
            matches.into_iter().map(String::from).collect(),
        )),
    }
}

/// generate a unique issue ID.
pub fn generate_issue_id(config: &Config, issues_dir: &Path) -> Result<String> {
    let charset: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::rng();

    for _ in 0..20 {
        let suffix: String = (0..config.id_len)
            .map(|_| {
                let idx = rng.random_range(0..charset.len());
                charset[idx] as char
            })
            .collect();

        let id = format!("{}-{}", config.id_prefix, suffix);
        let path = issues_dir.join(format!("{}.md", id));

        if !path.exists() {
            return Ok(id);
        }
    }

    Err(BrdError::Other(
        "failed to generate unique ID after 20 attempts".to_string(),
    ))
}

/// split content into frontmatter and body.
fn split_frontmatter(content: &str) -> Result<(&str, &str)> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Err(BrdError::ParseError(
            "issue".to_string(),
            "missing frontmatter delimiter".to_string(),
        ));
    }

    let after_first = &content[3..];
    let end_pos = after_first.find("\n---").ok_or_else(|| {
        BrdError::ParseError(
            "issue".to_string(),
            "missing closing frontmatter delimiter".to_string(),
        )
    })?;

    let frontmatter = after_first[..end_pos].trim();
    let body = after_first[end_pos + 4..].trim_start_matches('\n');

    Ok((frontmatter, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_issue() {
        let content = r#"---
schema_version: 2
id: "test-abc123"
title: "Test issue"
priority: P1
status: todo
deps: []
created_at: 2025-12-25T12:00:00Z
updated_at: 2025-12-25T12:00:00Z
---

This is the body.
"#;

        let issue = Issue::parse(content).unwrap();
        assert_eq!(issue.frontmatter.id, "test-abc123");
        assert_eq!(issue.frontmatter.title, "Test issue");
        assert_eq!(issue.frontmatter.priority, Priority::P1);
        assert_eq!(issue.frontmatter.status, Status::Todo);
        assert!(issue.frontmatter.tags.is_empty());
        assert!(issue.body.contains("This is the body"));
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::P0 < Priority::P1);
        assert!(Priority::P1 < Priority::P2);
        assert!(Priority::P2 < Priority::P3);
    }

    fn make_test_issues(ids: &[&str]) -> std::collections::HashMap<String, Issue> {
        ids.iter()
            .map(|id| {
                let issue = Issue::new(
                    id.to_string(),
                    format!("test issue {}", id),
                    Priority::P2,
                    vec![],
                );
                (id.to_string(), issue)
            })
            .collect()
    }

    #[test]
    fn test_resolve_issue_id_exact_match() {
        let issues = make_test_issues(&["brd-abc1", "brd-abc2", "brd-xyz1"]);
        let result = resolve_issue_id("brd-abc1", &issues);
        assert_eq!(result.unwrap(), "brd-abc1");
    }

    #[test]
    fn test_resolve_issue_id_exact_match_takes_precedence() {
        // if "abc" is an exact match, it should be returned even if "abcd" also exists
        let issues = make_test_issues(&["abc", "abcd", "abcde"]);
        let result = resolve_issue_id("abc", &issues);
        assert_eq!(result.unwrap(), "abc");
    }

    #[test]
    fn test_resolve_issue_id_partial_suffix() {
        let issues = make_test_issues(&["brd-abc1", "brd-xyz2"]);
        // "abc1" is a suffix of "brd-abc1"
        let result = resolve_issue_id("abc1", &issues);
        assert_eq!(result.unwrap(), "brd-abc1");
    }

    #[test]
    fn test_resolve_issue_id_partial_contains() {
        let issues = make_test_issues(&["brd-abc1", "brd-xyz2"]);
        // "abc" is contained in "brd-abc1"
        let result = resolve_issue_id("abc", &issues);
        assert_eq!(result.unwrap(), "brd-abc1");
    }

    #[test]
    fn test_resolve_issue_id_ambiguous() {
        let issues = make_test_issues(&["brd-abc1", "brd-abc2", "brd-xyz1"]);
        // "abc" matches both brd-abc1 and brd-abc2
        let result = resolve_issue_id("abc", &issues);
        match result {
            Err(crate::error::BrdError::AmbiguousId(partial, candidates)) => {
                assert_eq!(partial, "abc");
                assert_eq!(candidates.len(), 2);
                assert!(candidates.contains(&"brd-abc1".to_string()));
                assert!(candidates.contains(&"brd-abc2".to_string()));
            }
            _ => panic!("expected AmbiguousId error"),
        }
    }

    #[test]
    fn test_resolve_issue_id_not_found() {
        let issues = make_test_issues(&["brd-abc1", "brd-xyz2"]);
        let result = resolve_issue_id("nonexistent", &issues);
        match result {
            Err(crate::error::BrdError::IssueNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            _ => panic!("expected IssueNotFound error"),
        }
    }

    #[test]
    fn test_resolve_issue_id_empty_issues() {
        let issues: std::collections::HashMap<String, Issue> = std::collections::HashMap::new();
        let result = resolve_issue_id("anything", &issues);
        assert!(matches!(
            result,
            Err(crate::error::BrdError::IssueNotFound(_))
        ));
    }

    #[test]
    fn test_parse_missing_optional_fields() {
        // minimal issue with only required fields
        let content = r#"---
schema_version: 2
id: "test-minimal"
title: "Minimal issue"
priority: P2
status: todo
created_at: 2025-12-25T12:00:00Z
updated_at: 2025-12-25T12:00:00Z
---
"#;

        let issue = Issue::parse(content).unwrap();
        assert_eq!(issue.frontmatter.id, "test-minimal");
        assert_eq!(issue.frontmatter.title, "Minimal issue");
        assert_eq!(issue.frontmatter.priority, Priority::P2);
        assert_eq!(issue.frontmatter.status, Status::Todo);
        // optional fields should have defaults
        assert!(issue.frontmatter.deps.is_empty());
        assert!(issue.frontmatter.tags.is_empty());
        assert!(issue.frontmatter.owner.is_none());
        assert!(issue.frontmatter.issue_type.is_none());
        assert!(issue.frontmatter.acceptance.is_empty());
        assert!(issue.body.is_empty());
    }

    #[test]
    fn test_parse_with_all_optional_fields() {
        let content = r#"---
schema_version: 2
id: "test-full"
title: "Full issue"
priority: P0
status: doing
type: design
deps:
  - dep-1
  - dep-2
tags:
  - bug
  - urgent
owner: agent-one
created_at: 2025-12-25T12:00:00Z
updated_at: 2025-12-25T13:00:00Z
acceptance:
  - first criterion
  - second criterion
---

issue body here.
"#;

        let issue = Issue::parse(content).unwrap();
        assert_eq!(issue.frontmatter.id, "test-full");
        assert_eq!(issue.frontmatter.priority, Priority::P0);
        assert_eq!(issue.frontmatter.status, Status::Doing);
        assert_eq!(issue.frontmatter.issue_type, Some(IssueType::Design));
        assert_eq!(issue.frontmatter.deps, vec!["dep-1", "dep-2"]);
        assert_eq!(issue.frontmatter.tags, vec!["bug", "urgent"]);
        assert_eq!(issue.frontmatter.owner, Some("agent-one".to_string()));
        assert_eq!(
            issue.frontmatter.acceptance,
            vec!["first criterion", "second criterion"]
        );
        assert_eq!(issue.body, "issue body here.\n");
    }

    #[test]
    fn test_parse_empty_body() {
        let content = r#"---
schema_version: 2
id: "test-nobody"
title: "No body"
priority: P3
status: done
created_at: 2025-12-25T12:00:00Z
updated_at: 2025-12-25T12:00:00Z
---
"#;

        let issue = Issue::parse(content).unwrap();
        assert!(issue.body.is_empty());
    }

    #[test]
    fn test_parse_whitespace_only_body() {
        let content = r#"---
schema_version: 2
id: "test-whitespace"
title: "Whitespace body"
priority: P3
status: todo
created_at: 2025-12-25T12:00:00Z
updated_at: 2025-12-25T12:00:00Z
---



"#;

        let issue = Issue::parse(content).unwrap();
        // body preserves whitespace after the frontmatter delimiter
        assert!(issue.body.trim().is_empty());
    }

    #[test]
    fn test_parse_malformed_missing_frontmatter_start() {
        let content = r#"id: "test"
title: "No delimiters"
---
"#;

        let result = Issue::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_malformed_missing_frontmatter_end() {
        let content = r#"---
id: "test"
title: "No end delimiter"
"#;

        let result = Issue::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_malformed_invalid_yaml() {
        let content = r#"---
id: [unclosed bracket
title: "Bad yaml"
---
"#;

        let result = Issue::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_malformed_missing_required_field() {
        // missing title field
        let content = r#"---
schema_version: 2
id: "test-notitle"
priority: P2
status: todo
created_at: 2025-12-25T12:00:00Z
updated_at: 2025-12-25T12:00:00Z
---
"#;

        let result = Issue::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_malformed_invalid_priority() {
        let content = r#"---
schema_version: 2
id: "test-badpri"
title: "Bad priority"
priority: P9
status: todo
created_at: 2025-12-25T12:00:00Z
updated_at: 2025-12-25T12:00:00Z
---
"#;

        let result = Issue::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_malformed_invalid_status() {
        let content = r#"---
schema_version: 2
id: "test-badstatus"
title: "Bad status"
priority: P2
status: invalid
created_at: 2025-12-25T12:00:00Z
updated_at: 2025-12-25T12:00:00Z
---
"#;

        let result = Issue::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_serialization() {
        let content = r#"---
schema_version: 2
id: "test-roundtrip"
title: "Roundtrip test"
priority: P1
status: doing
type: meta
deps:
  - dep-a
  - dep-b
tags:
  - feature
owner: test-agent
created_at: 2025-12-25T12:00:00Z
updated_at: 2025-12-25T14:00:00Z
acceptance:
  - must work
  - must be fast
---

this is the body content.

with multiple paragraphs.
"#;

        // parse the original
        let issue = Issue::parse(content).unwrap();

        // serialize back to markdown
        let serialized = issue.to_markdown().unwrap();

        // parse the serialized version
        let reparsed = Issue::parse(&serialized).unwrap();

        // verify all fields match
        assert_eq!(issue.frontmatter.id, reparsed.frontmatter.id);
        assert_eq!(issue.frontmatter.title, reparsed.frontmatter.title);
        assert_eq!(issue.frontmatter.priority, reparsed.frontmatter.priority);
        assert_eq!(issue.frontmatter.status, reparsed.frontmatter.status);
        assert_eq!(issue.frontmatter.issue_type, reparsed.frontmatter.issue_type);
        assert_eq!(issue.frontmatter.deps, reparsed.frontmatter.deps);
        assert_eq!(issue.frontmatter.tags, reparsed.frontmatter.tags);
        assert_eq!(issue.frontmatter.owner, reparsed.frontmatter.owner);
        assert_eq!(issue.frontmatter.created_at, reparsed.frontmatter.created_at);
        assert_eq!(issue.frontmatter.updated_at, reparsed.frontmatter.updated_at);
        assert_eq!(issue.frontmatter.acceptance, reparsed.frontmatter.acceptance);
        assert_eq!(issue.body, reparsed.body);
    }

    #[test]
    fn test_roundtrip_minimal_issue() {
        // test roundtrip with minimal fields to ensure optional fields serialize correctly
        let issue = Issue::new(
            "test-new".to_string(),
            "New issue".to_string(),
            Priority::P2,
            vec![],
        );

        let serialized = issue.to_markdown().unwrap();
        let reparsed = Issue::parse(&serialized).unwrap();

        assert_eq!(issue.frontmatter.id, reparsed.frontmatter.id);
        assert_eq!(issue.frontmatter.title, reparsed.frontmatter.title);
        assert_eq!(issue.frontmatter.priority, reparsed.frontmatter.priority);
        assert_eq!(issue.frontmatter.status, reparsed.frontmatter.status);
        assert!(reparsed.frontmatter.tags.is_empty());
        assert!(reparsed.frontmatter.acceptance.is_empty());
        assert!(reparsed.frontmatter.owner.is_none());
    }

    #[test]
    fn test_generate_issue_id_format() {
        let config = Config {
            schema_version: 4,
            id_prefix: "test".to_string(),
            id_len: 4,
        };
        let temp_dir = tempfile::tempdir().unwrap();

        let id = generate_issue_id(&config, temp_dir.path()).unwrap();

        // should be "prefix-suffix" format
        assert!(id.starts_with("test-"));
        let parts: Vec<&str> = id.splitn(2, '-').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "test");
    }

    #[test]
    fn test_generate_issue_id_suffix_length() {
        let config = Config {
            schema_version: 4,
            id_prefix: "brd".to_string(),
            id_len: 6,
        };
        let temp_dir = tempfile::tempdir().unwrap();

        let id = generate_issue_id(&config, temp_dir.path()).unwrap();

        let suffix = id.strip_prefix("brd-").unwrap();
        assert_eq!(suffix.len(), 6);
    }

    #[test]
    fn test_generate_issue_id_charset() {
        let config = Config {
            schema_version: 4,
            id_prefix: "x".to_string(),
            id_len: 10,
        };
        let temp_dir = tempfile::tempdir().unwrap();

        // generate multiple IDs to check charset
        for _ in 0..10 {
            let id = generate_issue_id(&config, temp_dir.path()).unwrap();
            let suffix = id.strip_prefix("x-").unwrap();

            // all chars should be lowercase alphanumeric
            for c in suffix.chars() {
                assert!(
                    c.is_ascii_lowercase() || c.is_ascii_digit(),
                    "unexpected char in suffix: {}",
                    c
                );
            }
        }
    }

    #[test]
    fn test_generate_issue_id_avoids_collision() {
        let config = Config {
            schema_version: 4,
            id_prefix: "col".to_string(),
            id_len: 4,
        };
        let temp_dir = tempfile::tempdir().unwrap();

        // generate first ID and create a file for it
        let id1 = generate_issue_id(&config, temp_dir.path()).unwrap();
        let path1 = temp_dir.path().join(format!("{}.md", id1));
        std::fs::write(&path1, "exists").unwrap();

        // generate more IDs - they should all be different from id1
        for _ in 0..10 {
            let id2 = generate_issue_id(&config, temp_dir.path()).unwrap();
            assert_ne!(id1, id2, "generated same ID as existing file");
        }
    }
}
