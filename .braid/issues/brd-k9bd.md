---
schema_version: 8
id: brd-k9bd
title: 'design: explore programmatic interaction with claude code'
priority: P2
status: open
type: design
deps: []
owner: null
created_at: 2026-01-24T19:26:26.030067Z
---

## context

`brd agent send` is currently a stub. we need to understand how to send input to a running claude code process.

## questions to research

1. does claude code have an API or IPC mechanism for receiving input?
2. can we use stdin piping effectively with `--print` mode?
3. are there websocket/socket options?
4. what does the `--input-format stream-json` flag enable?
5. how do other tools interact with claude code programmatically?

## resources to check

- claude code documentation
- claude code --help output (already explored: has --input-format stream-json)
- anthropic developer docs
- github issues/discussions on claude code

## goal

determine the best approach for:
- sending messages to a running agent
- detecting when an agent is waiting for input
- gracefully interrupting an agent