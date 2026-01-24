---
schema_version: 8
id: brd-luak
title: 'design: extract shared test repo setup utilities'
priority: P1
status: done
type: design
deps: []
tags:
- refactor
owner: null
created_at: 2026-01-20T11:23:41.701992Z
started_at: 2026-01-22T21:05:33.78113Z
completed_at: 2026-01-22T21:10:38.329936Z
---

## problem

identical `create_test_repo()` function is copy-pasted across 7 test modules (~480 lines total):
- `done.rs:206-218`
- `skip.rs:49-61`
- `set.rs:102-114`
- `start.rs:473-490`
- `dep.rs:89-99`
- `add.rs:60-72`
- `agent.rs:812-827` (named `create_paths()`)

typical implementation:
```rust
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
```

## actual variations found

examined all 7 test modules - here's what differs:

**return types:**
- done.rs, skip.rs, set.rs, start.rs → `(TempDir, RepoPaths, Config)`
- dep.rs, add.rs, agent.rs → `(TempDir, RepoPaths)` (no config in tuple)

**extra setup:**
- start.rs creates `agent.toml` with `agent_id = "tester"`
- dep.rs splits into `create_braid_dir()` + `create_valid_config()` helpers
- agent.rs uses different directory structure (`.braid/` vs `.braid/issues/`)

**write_issue helpers (also duplicated):**
- done.rs, skip.rs: `write_issue(paths, config, id, status, owner)`
- set.rs: `write_issue(paths, config, id, priority)`
- start.rs: `write_issue(paths, config, id, priority, status, issue_type, owner)`
- dep.rs, add.rs: write raw YAML strings directly

**make_cli (identical in all 7):**
```rust
fn make_cli() -> Cli {
    Cli { json: false, repo: None, no_color: true, verbose: false, command: Command::Doctor }
}
```

## design proposal

create `src/test_utils.rs` (not under commands/) with:

### 1. TestRepo builder

```rust
pub struct TestRepo {
    _dir: TempDir,
    pub paths: RepoPaths,
    pub config: Config,
}

impl TestRepo {
    pub fn new() -> TestRepoBuilder {
        TestRepoBuilder::default()
    }
}

pub struct TestRepoBuilder {
    agent_id: Option<String>,
    config: Config,
}

impl TestRepoBuilder {
    pub fn with_agent(mut self, id: &str) -> Self {
        self.agent_id = Some(id.to_string());
        self
    }

    pub fn with_issues_branch(mut self, branch: &str) -> Self {
        self.config.issues_branch = Some(branch.to_string());
        self
    }

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
            ).unwrap();
        }

        TestRepo { _dir: dir, paths, config: self.config }
    }
}
```

### 2. IssueBuilder for write_issue

```rust
pub struct IssueBuilder<'a> {
    repo: &'a TestRepo,
    id: String,
    title: Option<String>,
    priority: Priority,
    status: Status,
    issue_type: Option<IssueType>,
    owner: Option<String>,
    deps: Vec<String>,
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
        }
    }

    pub fn status(mut self, s: Status) -> Self { self.status = s; self }
    pub fn priority(mut self, p: Priority) -> Self { self.priority = p; self }
    pub fn owner(mut self, o: &str) -> Self { self.owner = Some(o.to_string()); self }
    pub fn issue_type(mut self, t: IssueType) -> Self { self.issue_type = Some(t); self }
    pub fn deps(mut self, d: &[&str]) -> Self { self.deps = d.iter().map(|s| s.to_string()).collect(); self }

    pub fn create(self) {
        let mut issue = Issue::new(
            self.id.clone(),
            self.title.unwrap_or_else(|| format!("issue {}", self.id)),
            self.priority,
            self.deps,
        );
        issue.frontmatter.status = self.status;
        issue.frontmatter.issue_type = self.issue_type;
        issue.frontmatter.owner = self.owner;
        let path = self.repo.paths.issues_dir(&self.repo.config).join(format!("{}.md", self.id));
        issue.save(&path).unwrap();
    }
}

// convenience method on TestRepo
impl TestRepo {
    pub fn issue(&self, id: &str) -> IssueBuilder<'_> {
        IssueBuilder::new(self, id)
    }
}
```

### 3. test_cli() function

```rust
pub fn test_cli() -> Cli {
    Cli {
        json: false,
        repo: None,
        no_color: true,
        verbose: false,
        command: Command::Doctor,
    }
}
```

## usage example

```rust
use crate::test_utils::{TestRepo, test_cli};

#[test]
fn test_done_sets_status() {
    let repo = TestRepo::new().with_agent("tester").build();
    repo.issue("brd-aaaa").status(Status::Doing).owner("tester").create();

    cmd_done(&test_cli(), &repo.paths, "brd-aaaa", false, &[], true).unwrap();

    let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
    assert_eq!(issues.get("brd-aaaa").unwrap().status(), Status::Done);
}
```

## migration path

1. add `src/test_utils.rs` with `#[cfg(test)]`
2. update one command's tests as proof of concept
3. migrate remaining commands incrementally
4. delete per-module helpers as they become unused
