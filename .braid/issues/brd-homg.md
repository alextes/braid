---
schema_version: 4
id: brd-homg
title: add brd rm command to delete issues
priority: P2
status: doing
deps: []
owner: agent-two
created_at: 2025-12-27T13:45:21.623041Z
updated_at: 2025-12-27T14:59:06.389579Z
acceptance:
- brd rm <id> deletes the issue file
- refuses if issue is in doing status (safety)
- dual-write support for agent worktrees
---

sometimes you want an issue to disappear entirely. manual deletion works but a command is nicer.