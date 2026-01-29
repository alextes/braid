---
schema_version: 9
id: brd-k788
title: 'TUI: clarify dependency list visual language (colors and markers)'
priority: P2
status: open
type: design
deps: []
tags:
- tui
- ux
owner: null
created_at: 2026-01-28T15:45:15.434166Z
---

the colors and markers in the dependency list are confusing.

## observed confusion

in the detail pane dependency list:
- green bold with green `>` - what does this mean? is it good? done?
- red text - is this the blocker? or skip status? or blocked itself?
- the visual language is unclear

## questions to resolve

1. what does each color mean?
   - green = done?
   - red = open/blocking?
   - dim = resolved?

2. what does each marker mean?
   - `>` = selected for preview
   - `-` = not selected
   - should there be status markers too?

3. should we use:
   - colors only (current approach, confusing)
   - symbols only (✓, ○, ✗)
   - both (✓ green, ○ yellow, etc.)
   - text labels (done), (open), (skip)

## goal

user should instantly understand:
- which deps are resolved (done/skip)
- which deps are open (potential blockers)
- which dep is currently selected for preview

## related
- brd-8zb7: detail pane blocking status