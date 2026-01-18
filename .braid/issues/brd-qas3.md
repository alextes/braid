---
schema_version: 7
id: brd-qas3
title: 'design: interact with running claude agent'
priority: P2
status: open
type: design
deps: []
owner: null
created_at: 2026-01-18T14:18:21.998249Z
updated_at: 2026-01-18T14:20:29.727114Z
---

Design how users can interact with a claude agent that's working on an issue.

## open questions
- how to detect when agent needs input?
- how to send input to the agent?
- how to show agent progress/output?
- should interaction be synchronous or async?
- how does this work from CLI vs TUI?

## ideas to explore
- claude code's interactive mode
- stdin/stdout piping
- file-based communication
- websocket/IPC