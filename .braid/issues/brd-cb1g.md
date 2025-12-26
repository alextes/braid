---
schema_version: 2
id: brd-cb1g
title: add dual-write support to brd add command
priority: P2
status: done
deps: []
created_at: 2025-12-26T20:11:16.560141Z
updated_at: 2025-12-26T21:31:54.947749Z
acceptance:
- detect when running from agent worktree (worktree_root != control_root)
- write issue to both control root and local worktree
- matches existing dual-write behavior in start/done commands
---

currently brd add only writes to the control root. when running from an agent worktree, the new issue doesn't appear in the local .braid/issues/ directory, so agents can't commit issue creation.

brd start and brd done already implement dual-write - they write to both control root (for immediate visibility) and local worktree (for git commits).

brd add should do the same when paths.worktree_root != paths.control_root.