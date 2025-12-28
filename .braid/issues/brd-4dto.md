---
schema_version: 4
id: brd-4dto
title: add unit tests for start/done/skip commands
priority: P2
status: doing
deps: []
tags:
- testing
owner: agent-three
created_at: 2025-12-28T08:49:06.106631Z
updated_at: 2025-12-28T18:35:43.908711Z
---

core workflow commands have no unit tests.

## start command
- sets status to Doing
- sets owner to agent_id
- auto-picks next ready if no ID given
- skips meta issues in auto-pick
- prevents starting already-doing issues
- force flag behavior

## done command
- sets status to Done
- clears owner
- dual-write behavior

## skip command
- sets status to Skip
- clears owner

## shared test cases
- issue not found error
- ambiguous ID error
- file write verification