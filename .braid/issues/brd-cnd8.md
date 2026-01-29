---
schema_version: 9
id: brd-cnd8
title: 'TUI: spawn and manage agents'
priority: P2
status: done
deps:
- brd-xnxw
tags:
- tui
- agent
owner: null
created_at: 2026-01-18T14:18:27.121773Z
completed_at: 2026-01-29T13:45:10.836079Z
---

Add TUI support for spawning and managing ephemeral agents.

## depends on
- CLI agent spawn functionality (brd-xnxw)

## features
- hotkey to spawn agent for selected issue
- show agent status in sidebar
- switch between agent views
- handle agent input requests in TUI
- cleanup controls