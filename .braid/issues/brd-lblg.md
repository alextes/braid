---
brd: 1
id: brd-lblg
title: design better agent ID fallback
priority: P2
status: todo
deps: []
created_at: 2025-12-26T08:47:02.778179Z
updated_at: 2025-12-26T08:47:02.778179Z
---

current fallback for agent ID is `<hostname>:<pid>` which is not great.

current resolution order:
1. BRD_AGENT_ID env var
2. .braid/agent.toml
3. fallback: hostname:pid

ideas:
- prompt interactively if no agent ID is configured
- require explicit setup before first use (like git config user.name)
- distinguish between human users and automated agents

humans should be able to work on issues too, using their own worktree. need a clear way to identify human vs agent.
