---
schema_version: 4
id: brd-ucso
title: 'design: external-repo mode for tracking issues in a separate repository'
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2025-12-30T15:39:59.026761Z
updated_at: 2025-12-30T15:40:47.192506Z
---

Track braid issues in a completely separate repository.

## Motivation

Three core use cases:
1. **Separation of concerns**: Keep issue history separate from code history
2. **Multi-repo coordination**: Central issues repo for multiple code repos
3. **Privacy**: Private issues repo with public code repo

After this mode, braid has three core modes covering ~80% of workflows:
- `git-native`: issues on main (simple, solo/small teams)
- `local-sync`: issues on sync branch (local agent coordination)
- `external-repo`: issues in separate repo (separation, privacy, multi-repo)

## Design

### Configuration

New config field in code repo's `.braid/config.toml`:
```toml
schema_version = 5
id_prefix = "mypr"
id_len = 4
issues_repo = "../myproject-issues"  # relative or absolute path
```

Environment variable override: `BRD_ISSUES_REPO`

### How it works

1. brd resolves `issues_repo` path (relative to worktree root)
2. Goes to that repo, loads **its** config
3. Uses that repo's `issues_dir()` logic

The external repo is autonomous - if it uses local-sync internally, brd follows that.

### Multi-repo via branches

For separate issues per code repo using one external repo:
- Create worktrees of external repo on different branches
- Point each code repo to its worktree

```
external-issues/           # main branch (shared)
external-issues-frontend/  # worktree on frontend-issues branch
external-issues-backend/   # worktree on backend-issues branch
```

Frontend code repo: `issues_repo = "../external-issues-frontend"`
Backend code repo: `issues_repo = "../external-issues-backend"`

### User workflow (v1)

```bash
# 1. Create external issues repo (user does both steps manually for v1)
mkdir myproject-issues && cd myproject-issues
git init && brd init
git add -A && git commit -m "init braid"
git remote add origin git@github.com:me/myproject-issues.git
git push -u origin main

# 2. Point code repo to it
cd ../myproject
brd mode external-repo ../myproject-issues

# 3. Use brd normally
brd add "my first issue"
brd ls
```

### Agent worktrees

Work as today. Agent worktrees copy `.braid/config.toml` from the code repo branch, which includes `issues_repo`. No changes needed.

## Future improvements (separate issues)

- `brd mode external-repo --init <path>` to create + init external repo
- `gh repo create` integration if gh CLI installed
- `issues_branch` config to auto-manage worktrees for branch separation

## Documentation updates needed

- README mode comparison table (add external-repo column)
- New guide for setting up external-repo mode
- Update `brd mode --help`

## Output

Implementation plan with resulting issues.