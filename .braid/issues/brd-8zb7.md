---
schema_version: 9
id: brd-8zb7
title: 'TUI: detail pane dependency list should show blocking status clearly'
priority: P2
status: open
type: design
deps:
- brd-k788
tags:
- tui
- ux
owner: null
created_at: 2026-01-28T15:44:47.129653Z
---

the detail pane shows dependencies but doesn't make it clear which are blocking.

## current state

```
Dependencies:
> brd-yci2
- brd-1i52
- brd-of8m
```

- `>` indicates selected for preview, not status
- no indication of done vs open
- if issue is BLOCKED, you have to arrow through each dep to find the blocker

## problem

user sees `State: BLOCKED` but can't tell at a glance which dep is causing it.

## ideas

### option 1: show status inline
```
Dependencies:
  ✓ brd-yci2 (done)
> ○ brd-1i52 (open)   ← this is the blocker
  ✓ brd-of8m (done)
```

### option 2: separate blockers from resolved
```
Blocking (1):
> brd-1i52 (open)

Resolved (2):
  brd-yci2 (done)
  brd-of8m (done)
```

### option 3: only show blockers when blocked
if issue is BLOCKED, prominently show just the open deps:
```
Blocked by:
> brd-1i52: design: track PR URL in issue frontmatter
```

## related
- CLI `brd show` already does this well with `(done)` / `(open)` and dimming