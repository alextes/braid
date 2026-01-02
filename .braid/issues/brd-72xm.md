---
schema_version: 6
id: brd-72xm
title: update write commands to use branch switching in issues-branch mode
priority: P2
status: skip
deps:
- brd-8vcu
owner: null
created_at: 2025-12-28T23:59:41.200534Z
updated_at: 2025-12-30T15:54:10.542931Z
---

Update all commands that write to issues to use the branch switching infrastructure when in issues-branch mode.

## Commands to update

- `brd add` - create new issue
- `brd start` - claim issue
- `brd done` - complete issue
- `brd skip` - skip issue
- `brd rm` - remove issue
- `brd dep add/rm` - modify dependencies

## Pattern

Each command should check `config.issues_branch` and if set, wrap the write operation in `with_issues_branch()`.

## Acceptance criteria
- [ ] All write commands work in issues-branch mode
- [ ] Commits go to the issues branch, not current branch
- [ ] Original branch state is preserved