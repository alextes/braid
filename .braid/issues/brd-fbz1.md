---
schema_version: 6
id: brd-fbz1
title: update brd done to auto-push when config.auto_push is true
priority: P2
status: done
deps:
- brd-zeqv
owner: null
created_at: 2025-12-30T17:08:56.720094Z
updated_at: 2025-12-30T20:09:54.594101Z
---

Add auto-push behavior to `brd done` when `config.auto_push` is enabled.

## Current behavior
- `brd done` just marks issue as done and saves file
- No commit or push happens
- User must manually commit/push or run `brd sync`

## New behavior
- If `config.auto_push = true`: commit and push after marking done
- If `config.auto_push = false`: just save file (current behavior)

## Implementation

In `src/commands/done.rs`:
1. After saving the issue file, check `config.auto_push`
2. If true:
   - For git-native: `git add .braid && git commit && git push`
   - For local-sync: commit in issues worktree, push if remote exists
3. Add `--no-push` flag to skip even when auto_push is enabled

## Notes
- This enables the "always in sync" workflow for teams
- Matches the symmetry with auto_pull on start