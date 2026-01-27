---
schema_version: 8
id: brd-tfru
title: 'design: improve brd agent logs formatting'
priority: P2
status: open
type: design
deps:
- brd-hixx
tags:
- agents
owner: null
created_at: 2026-01-27T16:45:03.493678Z
---

## context

`brd agent logs` currently does basic pretty-printing of stream-json events,
but the output can still be hard to follow for longer sessions.

## questions to explore

- what information is most useful to show vs hide?
- how should tool calls be formatted? (currently just `[tool: Name]`)
- should we show timestamps?
- should there be a compact vs verbose mode?
- how to handle long tool outputs (truncate? collapse?)
- color coding for different event types?

## deliverable

write up 2-3 formatting approaches with trade-offs, then pick one to implement.