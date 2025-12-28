---
schema_version: 4
id: brd-af2c
title: add unit tests for add command
priority: P2
status: doing
deps: []
tags:
- testing
owner: alextes
created_at: 2025-12-28T08:49:05.99492Z
updated_at: 2025-12-28T17:44:42.858051Z
---

the add command has complex logic for issue creation but no unit tests.

## logic to test
- ID generation with config
- title validation (non-empty)
- priority parsing
- type parsing (design/meta)
- deps parsing and resolution
- tags parsing
- acceptance criteria parsing
- body handling
- dual-write to control root

## test cases
- minimal add (just title)
- full add (all options)
- invalid priority handling
- invalid type handling
- dep resolution errors