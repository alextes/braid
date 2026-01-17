---
schema_version: 6
id: brd-xrqb
title: 'design: expand brd set to support more fields'
priority: P2
status: todo
type: design
deps: []
owner: null
created_at: 2026-01-05T21:28:54.567535Z
updated_at: 2026-01-05T21:28:54.567535Z
---

## Context

`brd set` currently only supports priority. The help text says "(priority, tag)" but tag isn't implemented.

## Questions to explore

1. **What fields should be settable?**
   - priority ✓ (implemented)
   - tag (+add, -remove syntax?)
   - status (todo, doing, done, skip)
   - owner
   - type (design, meta, or clear)
   - title

2. **Syntax options for tags:**
   ```bash
   brd set <id> tag bug           # add tag
   brd set <id> tag +bug          # explicit add
   brd set <id> tag -bug          # remove tag
   brd set <id> tag bug,urgent    # multiple
   ```

3. **Should we support clearing fields?**
   ```bash
   brd set <id> owner --clear
   brd set <id> type --clear
   ```

4. **Batch updates?**
   ```bash
   brd set <id> priority P1 tag +urgent   # multiple fields
   brd set <id1> <id2> <id3> priority P2  # multiple issues
   ```

## Considerations

- Keep it simple - don't over-engineer
- Single field at a time is probably fine
- Tags need add/remove semantics, others are replace