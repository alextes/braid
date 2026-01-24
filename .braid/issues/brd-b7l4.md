---
schema_version: 8
id: brd-b7l4
title: TUI Dashboard View
priority: P2
status: open
type: design
deps: []
tags:
- tui
owner: null
created_at: 2026-01-17T10:36:01.360526Z
---

Explore adding a dashboard-style view as the first view in the TUI.

## Vision
A stats/overview dashboard showing the health and activity of the braid project at a glance.

## Potential Stats
- Issue counts by status (open/doing/done/skip)
- Issue counts by priority
- Active agents/worktrees count
- Recent activity (issues started/completed today/this week)

## Advanced Ideas
- Over-time charts (issues completed per day/week)
- Live agent status (which agents are working, what they're working on)
- Stale issue detection (doing issues with no recent activity)
- Dependency health (blocked issues, missing deps)

## Navigation
- View 1: Dashboard (stats overview)
- View 2: Live issues list (current implementation goal)

## References
- Similar to GitHub project insights or Jira dashboards