---
schema_version: 6
id: brd-obsv
title: prompt to update AGENTS.md after mode switch
priority: P2
status: skip
deps: []
owner: null
created_at: 2025-12-29T08:25:39.070202Z
updated_at: 2025-12-29T23:29:57.184156Z
---

After switching modes, the AGENTS.md block may have stale instructions for the old mode.

## Problem

1. User has AGENTS.md with git-native instructions
2. User runs `brd mode local-sync`
3. AGENTS.md still tells agents to use git-native workflow
4. Agents follow wrong instructions

## Solution

After mode switch, prompt user to update AGENTS.md:

```
Switched to local-sync mode.

Note: Run `brd agents inject` to update AGENTS.md with local-sync instructions.
```

Or even better, ask if they want to do it automatically:

```
Switched to local-sync mode.

Update AGENTS.md with local-sync instructions? [Y/n]:
```

## Related

- brd-jnin (agent worktree rebase warning)