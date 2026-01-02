---
schema_version: 6
id: brd-vgox
title: 'tui: inline filter mode instead of dialog'
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-28T23:55:34.871009Z
updated_at: 2025-12-30T16:21:11.276518Z
---

Change the filter UX in the All pane from a modal dialog to inline filtering:

- When All pane is active, pressing `/` enters filter mode
- Keystrokes immediately filter by title (shown inline, not in dialog)
- Enter confirms filter and returns to normal navigation (filter stays active)
- Esc clears the filter and exits filter mode
- 1-4 toggle status filters directly from normal mode (no dialog needed)

This is more vim-like and reduces friction for quick filtering.