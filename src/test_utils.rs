//! shared test utilities for braid commands
//!
//! provides TestRepo builder for consistent test setup across all command tests.

use std::fs;

use tempfile::{TempDir, tempdir};

use crate::cli::{Cli, Command};
use crate::config::Config;
use crate::issue::{Issue, IssueType, Priority, Status};
use crate::repo::RepoPaths;

/// a test repository with all necessary directories and config
pub struct TestRepo {
    _dir: TempDir,
    pub paths: RepoPaths,
    pub config: Config,
}

impl TestRepo {
    /// create a builder for configuring the test repo
    pub fn builder() -> TestRepoBuilder {
        TestRepoBuilder::default()
    }

    /// create an issue builder for this repo
    pub fn issue(&self, id: &str) -> IssueBuilder<'_> {
        IssueBuilder::new(self, id)
    }
}

impl Default for TestRepo {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// builder for TestRepo with optional configuration
#[derive(Default)]
pub struct TestRepoBuilder {
    agent_id: Option<String>,
    config: Config,
}

impl TestRepoBuilder {
    /// set up as an agent worktree with the given agent id
    pub fn with_agent(mut self, id: &str) -> Self {
        self.agent_id = Some(id.to_string());
        self
    }

    /// configure issues-branch mode
    pub fn with_issues_branch(mut self, branch: &str) -> Self {
        self.config.issues_branch = Some(branch.to_string());
        self
    }

    /// build the test repo, creating all directories and files
    pub fn build(self) -> TestRepo {
        let dir = tempdir().unwrap();
        let paths = RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        };
        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        fs::create_dir_all(paths.braid_dir().join("issues")).unwrap();
        self.config.save(&paths.config_path()).unwrap();

        if let Some(agent_id) = &self.agent_id {
            fs::write(
                paths.braid_dir().join("agent.toml"),
                format!("agent_id = \"{}\"\n", agent_id),
            )
            .unwrap();
        }

        TestRepo {
            _dir: dir,
            paths,
            config: self.config,
        }
    }
}

/// builder for creating test issues
pub struct IssueBuilder<'a> {
    repo: &'a TestRepo,
    id: String,
    title: Option<String>,
    priority: Priority,
    status: Status,
    issue_type: Option<IssueType>,
    owner: Option<String>,
    deps: Vec<String>,
    tags: Vec<String>,
}

impl<'a> IssueBuilder<'a> {
    pub fn new(repo: &'a TestRepo, id: &str) -> Self {
        Self {
            repo,
            id: id.to_string(),
            title: None,
            priority: Priority::P2,
            status: Status::Open,
            issue_type: None,
            owner: None,
            deps: vec![],
            tags: vec![],
        }
    }

    pub fn title(mut self, t: &str) -> Self {
        self.title = Some(t.to_string());
        self
    }

    pub fn status(mut self, s: Status) -> Self {
        self.status = s;
        self
    }

    pub fn priority(mut self, p: Priority) -> Self {
        self.priority = p;
        self
    }

    pub fn owner(mut self, o: &str) -> Self {
        self.owner = Some(o.to_string());
        self
    }

    pub fn issue_type(mut self, t: IssueType) -> Self {
        self.issue_type = Some(t);
        self
    }

    pub fn deps(mut self, d: &[&str]) -> Self {
        self.deps = d.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn tags(mut self, t: &[&str]) -> Self {
        self.tags = t.iter().map(|s| s.to_string()).collect();
        self
    }

    /// create the issue file in the test repo
    pub fn create(self) -> Issue {
        let mut issue = Issue::new(
            self.id.clone(),
            self.title.unwrap_or_else(|| format!("issue {}", self.id)),
            self.priority,
            self.deps,
        );
        issue.frontmatter.status = self.status;
        issue.frontmatter.issue_type = self.issue_type;
        issue.frontmatter.owner = self.owner;
        issue.frontmatter.tags = self.tags;

        let path = self
            .repo
            .paths
            .issues_dir(&self.repo.config)
            .join(format!("{}.md", self.id));
        issue.save(&path).unwrap();
        issue
    }
}

/// create a standard Cli for tests (json: false, no_color: true, verbose: false)
pub fn test_cli() -> Cli {
    Cli {
        json: false,
        repo: None,
        no_color: true,
        verbose: false,
        command: Command::Doctor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_creates_directories() {
        let repo = TestRepo::builder().build();
        assert!(repo.paths.brd_common_dir.exists());
        assert!(repo.paths.braid_dir().join("issues").exists());
        assert!(repo.paths.config_path().exists());
    }

    #[test]
    fn test_repo_with_agent() {
        let repo = TestRepo::builder().with_agent("tester").build();
        let agent_toml = repo.paths.braid_dir().join("agent.toml");
        assert!(agent_toml.exists());
        let content = fs::read_to_string(agent_toml).unwrap();
        assert!(content.contains("agent_id = \"tester\""));
    }

    #[test]
    fn test_issue_builder() {
        let repo = TestRepo::builder().build();
        let issue = repo
            .issue("brd-test")
            .status(Status::Doing)
            .priority(Priority::P1)
            .owner("alice")
            .create();

        assert_eq!(issue.id(), "brd-test");
        assert_eq!(issue.status(), Status::Doing);
        assert_eq!(issue.priority(), Priority::P1);
        assert_eq!(issue.frontmatter.owner, Some("alice".to_string()));
    }

    #[test]
    fn test_issue_builder_with_deps() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-dep1").create();
        let issue = repo.issue("brd-child").deps(&["brd-dep1"]).create();

        assert_eq!(issue.frontmatter.deps, vec!["brd-dep1".to_string()]);
    }
}
