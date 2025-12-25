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
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Todo => write!(f, "todo"),
            Status::Doing => write!(f, "doing"),
            Status::Done => write!(f, "done"),
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
            _ => Err(BrdError::ParseError(
                "status".to_string(),
                format!("invalid status: {s}"),
            )),
        }
    }
}

/// the frontmatter of an issue file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueFrontmatter {
    pub brd: u32,
    pub id: String,
    pub title: String,
    pub priority: Priority,
    pub status: Status,
    #[serde(default)]
    pub deps: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
        let now = OffsetDateTime::now_utc();
        Self {
            frontmatter: IssueFrontmatter {
                brd: 1,
                id,
                title,
                priority,
                status: Status::Todo,
                deps,
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
        let (frontmatter_str, body) = split_frontmatter(content)?;
        let frontmatter: IssueFrontmatter = serde_yaml::from_str(frontmatter_str)
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

    pub fn deps(&self) -> &[String] {
        &self.frontmatter.deps
    }
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
brd: 1
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
        assert!(issue.body.contains("This is the body"));
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::P0 < Priority::P1);
        assert!(Priority::P1 < Priority::P2);
        assert!(Priority::P2 < Priority::P3);
    }
}
