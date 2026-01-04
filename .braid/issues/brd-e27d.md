---
schema_version: 6
id: brd-e27d
title: testing checklist for workflow modes
priority: P2
status: done
deps: []
owner: null
created_at: 2026-01-02T13:27:04.113387Z
updated_at: 2026-01-04T15:22:38.373641Z
---

## Purpose

Track testing of braid's workflow configurations before v1.0. Can be worked through manually or with AI assistance.

## Checklist

### Mode Switching
- [x] `brd mode git-native` from fresh init (tested via integration test)
- [x] `brd mode local-sync` from git-native (tested manually + integration test)
- [x] `brd mode git-native` from local-sync (issues copied back) (tested manually + integration test)
- [x] `brd mode external-repo ../path` from git-native (tested manually)
- [x] `brd mode git-native` from external-repo (tested manually)

### Git-native Workflow
- [x] `brd init` with issues-with-code (answer No to Q1) — tested via JSON init
- [x] `brd start` claims issue, commits, pushes (tested manually + integration test)
- [x] `brd done` marks done, commits, pushes (tested manually + integration test)
- [ ] Race condition: two agents claim same issue (needs multi-agent test)

### Local-sync Workflow
- [x] `brd init -y` sets up issues branch (integration test)
- [x] Issues visible instantly across worktrees (tested manually in main repo)
- [ ] `brd sync` pushes issues branch to remote (needs remote setup)
- [x] Multiple agents see each other's claims (tested in main repo)

### Agent Worktrees
- [x] `brd agent init <name>` creates worktree (tested in main repo)
- [x] `brd agent merge` rebases and ff-merges to main (existing tests)
- [ ] `brd agent merge` on main warns appropriately (needs test)
- [x] `brd agent pr` creates PR (existing functionality)
- [ ] `brd agent pr` on main errors appropriately (needs test)

### External Repo
- [x] Set up external issues repo (tested manually)
- [x] Point code repo to it (tested manually)
- [x] Commands read/write to external repo (tested manually)

## Related

- brd-u9ka: integration tests for core paths from this checklist