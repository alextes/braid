---
schema_version: 3
id: brd-u8vh
title: add command to set up agent worktree
priority: P1
status: done
deps: []
owner: null
created_at: 2025-12-26T08:49:36.259455Z
updated_at: 2025-12-26T12:02:44.944048Z
---

add a command to streamline setting up a new agent worktree. something like:

```bash
brd agent init <agent-name>
```

should:
1. create a git worktree (e.g. `git worktree add ../<agent-name> -b <agent-name>`)
2. create `.braid/agent.toml` with `agent_id = "<agent-name>"`
3. print instructions for how to use the new worktree

currently this requires manual steps:
1. `git worktree add ../agent-a agent-a-branch`
2. `echo 'agent_id = "agent-a"' > ../agent-a/.braid/agent.toml`

related: brd-lblg (agent ID design), brd-4osx (worktree protection)
