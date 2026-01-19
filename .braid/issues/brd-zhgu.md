---
schema_version: 8
id: brd-zhgu
title: add tests for agent.rs worktree operations
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-31T08:27:38.588314Z
started_at: 2026-01-02T21:11:42.06506Z
completed_at: 2026-01-02T21:11:42.06506Z
---

agent.rs has 916 lines but only 16 unit tests focused on name validation. Critical functionality is untested:

- Worktree creation and initialization
- Branch management  
- PR workflow integration
- Agent injection logic
- Ship operations

Risk: Agent workflow breakage could block development workflows.

Test areas needed:
- Worktree creation edge cases
- Branch naming and conflicts
- Init command variations
- Ship command logic (has integration tests but no unit tests)
- Error handling for git worktree failures