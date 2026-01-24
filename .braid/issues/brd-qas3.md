---
schema_version: 8
id: brd-qas3
title: 'design: interact with running claude agent'
priority: P2
status: open
type: design
deps: []
tags:
- agent
owner: null
created_at: 2026-01-18T14:18:21.998249Z
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