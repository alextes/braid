---
schema_version: 8
id: brd-0nbv
title: 'design: improve agent log formatting and readability'
priority: P2
status: open
type: design
deps: []
tags:
- agent
owner: null
created_at: 2026-01-26T09:01:20.237298Z
---

## context

spawned agents write stream-json logs to `.git/brd/sessions/<session>.log`. `brd agent logs` parses these but the output is hard to follow.

## current state

- logs are newline-delimited JSON with types: system, assistant, user (tool results)
- `print_event()` in agent_run.rs does minimal formatting:
  - extracts text from assistant messages
  - shows `[tool: <name>]` for tool use
  - skips metadata events

## goals

1. make agent activity easier to follow at a glance
2. show what the agent is doing (thinking, tool calls, results)
3. highlight important events (errors, waiting for input, completion)

## ideas to explore

- color coding by event type
- collapsible tool results (show summary, expand for full output)
- progress indicators (spinner for running, checkmarks for done)
- timing info (how long each tool call took)
- cost tracking display
- condensed vs verbose modes

## sample log structure

```json
{"type":"system","subtype":"init",...}
{"type":"assistant","message":{"content":[{"type":"text","text":"..."}]}}
{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{...}}]}}
{"type":"user","message":{"content":[{"type":"tool_result","content":"..."}]}}
```

## next steps

- review existing log file to understand full event taxonomy
- prototype different formatting approaches
- get feedback on what's most useful