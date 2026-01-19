---
schema_version: 8
id: brd-lgfn
title: add unit tests for mode.rs
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-31T08:27:38.129107Z
started_at: 2026-01-02T10:49:04.601327Z
completed_at: 2026-01-02T10:49:04.601327Z
---

mode.rs has 583 lines and 0 unit tests. This module handles critical functionality:

- Mode switching between default/sync-branch/external-repo
- Agent worktree rebase detection (find_agent_worktrees_needing_rebase)
- Config migration during mode changes
- cmd_mode_* functions

Risk: Silent failures in mode switching could leave repos in inconsistent states.

Test areas needed:
- Mode detection and switching logic
- External repo configuration
- Rebase detection edge cases
- Config migration paths during mode changes