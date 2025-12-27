---
brd: 1
id: brd-p1dp
title: implement claims system for multi-agent coordination
priority: P3
status: todo
deps: []
created_at: 2025-12-25T21:46:27.951363Z
updated_at: 2025-12-26T16:44:10.000000Z
acceptance:
- claim, release, reclaim, claims commands work
- claims filtering in ready/next
- lease expiry and renewal
---

**update (2025-12-26): this is now low priority.**

basic collision avoidance already works without the formal claims system:
- `brd start` sets `status: doing` + `owner: <agent-id>` in control root
- all agents see this immediately (shared control root)
- `brd start` refuses if issue is already being worked on
- `brd ready` / `brd next` only show todo issues

the claims system would add:
- **leases with expiry** — crashed agents don't block issues forever
- **`brd claims`** — visibility into active claims
- **`brd reclaim`** — steal expired claims

reconsider whether this complexity is needed before implementing. for small-scale multi-agent setups, the owner-based system is sufficient.

**known edge case with current system:**

if both main worktree and an agent worktree commit changes to the same issue file (via dual-write), and each commit also contains other unrelated changes they want to keep, rebasing will require manual merge resolution. the dual-write design assumes the agent will rebase onto main and handle conflicts then, but this could be confusing. a proper claims system with locking could prevent this scenario.
