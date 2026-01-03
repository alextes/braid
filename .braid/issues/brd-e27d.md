---
schema_version: 6
id: brd-e27d
title: testing checklist for workflow modes
priority: P2
status: doing
deps: []
owner: alextes
created_at: 2026-01-02T13:27:04.113387Z
updated_at: 2026-01-03T10:29:44.59383Z
---

## Purpose

Track testing of braid's workflow configurations before v1.0. Can be worked through manually or with AI assistance.

## Checklist

### Mode Switching
- [ ] `brd mode git-native` from fresh init
- [ ] `brd mode local-sync` from git-native
- [ ] `brd mode git-native` from local-sync (issues copied back)
- [ ] `brd mode external-repo ../path` from git-native
- [ ] `brd mode git-native` from external-repo

### Git-native Workflow
- [ ] `brd init` with issues-with-code (answer No to Q1)
- [ ] `brd start` claims issue, commits, pushes
- [ ] `brd done` marks done, commits, pushes
- [ ] Race condition: two agents claim same issue

### Local-sync Workflow
- [ ] `brd init -y` sets up issues branch
- [ ] Issues visible instantly across worktrees
- [ ] `brd sync` pushes issues branch to remote
- [ ] Multiple agents see each other's claims

### Agent Worktrees
- [ ] `brd agent init <name>` creates worktree
- [ ] `brd agent merge` rebases and ff-merges to main
- [ ] `brd agent merge` on main warns appropriately
- [ ] `brd agent pr` creates PR
- [ ] `brd agent pr` on main errors appropriately

### External Repo
- [ ] Set up external issues repo
- [ ] Point code repo to it
- [ ] Commands read/write to external repo

## Related

- brd-u9ka: integration tests for core paths from this checklist