---
schema_version: 8
id: brd-homg
title: add brd rm command to delete issues
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-27T13:45:21.623041Z
started_at: 2025-12-27T15:25:33.017681Z
completed_at: 2025-12-27T15:25:33.017681Z
acceptance:
- brd rm <id> deletes the issue file
- refuses if issue is in doing status (safety)
- dual-write support for agent worktrees
---

sometimes you want an issue to disappear entirely. manual deletion works but a command is nicer.