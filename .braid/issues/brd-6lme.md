---
schema_version: 4
id: brd-6lme
title: add git show/ls-tree helpers for reading from issues branch
priority: P2
status: skip
deps:
- brd-vwel
owner: null
created_at: 2025-12-28T23:58:33.758788Z
updated_at: 2025-12-30T15:54:10.500944Z
---

Add infrastructure to read issues directly from a branch without checkout.

## Changes

**src/repo.rs:**
```rust
/// List files in .braid/issues/ on a branch
pub fn list_issues_on_branch(&self, branch: &str) -> Result<Vec<String>>
// Uses: git ls-tree --name-only {branch}:.braid/issues/

/// Read issue content from a branch
pub fn read_issue_from_branch(&self, branch: &str, id: &str) -> Result<String>
// Uses: git show {branch}:.braid/issues/{id}.md
```

**src/issue.rs:**
- Update `load_issues()` to check config for `issues_branch`
- If set, use the new helpers instead of reading from filesystem

## Acceptance criteria
- [ ] Can list issues from a branch without checkout
- [ ] Can read issue content from a branch
- [ ] `brd ls` and `brd show` work in issues-branch mode