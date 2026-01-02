---
schema_version: 4
id: brd-lblg
title: design better agent ID fallback
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-26T08:47:02.778179Z
updated_at: 2025-12-26T15:43:31.396192Z
---

current fallback for agent ID is `<hostname>:<pid>` which is not great.

current resolution order:

1. BRD_AGENT_ID env var
2. .braid/agent.toml
3. fallback: hostname:pid

ideas:

- prompt interactively if no agent ID is configured
- require explicit setup before first use (like git config user.name)
- distinguish between human users and automated agents. double check whether there is need for this. main modes to work with brd are:
  - single agent only. there's only the main branch cloned. maybe feature branches are used maybe not. in any case only a single coding agent works on tasks. there's really no need to claim tasks even but for simplicity sake we can use the usual flow. tasks are picked up, worked on, and when finished closed.
  - human + agent. it may be the human executing the brd commands, it may be the agent. they work together still in a single repo. it may be possible the human claims a task and the agent claims one, and they both execute within the same worktree. generally though, both should be expected to be working together on the same task. for multiple tasks worked on in parallel, recommend setting up a worktree.
  - human + multiple agents. same as last. but now multiple worktrees exist that multiple agents work on in parallel. it is important to be able to track which agent has claimed which task.
- hostname:pid makes for an ugly name. find a better default. perhaps even default-user or agent-zero could simply function as the brd_agent_id, in which case maybe brd_user_id is most natural.
