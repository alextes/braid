---
schema_version: 8
id: brd-u360
title: 'brd ls: limit visible todo issues with indicator for more'
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2025-12-29T23:34:26.853547Z
started_at: 2025-12-30T16:38:38.831768Z
completed_at: 2025-12-30T16:38:38.831768Z
---

`brd ls` already limits done issues to recent ones. but the todo list can grow large too.

## idea

show top N todo issues (by priority, then age), with indicator:

```
brd-aaaa  P1  ...
brd-bbbb  P2  ...
brd-cccc  P2  ...
... and 35 more todo issues
```

or use `--all` to show everything.

## considerations

- what's a good default limit? 10? 20?
- should doing issues always show (probably yes)
- the footer already shows total counts, so user knows more exist

---

## analysis (agent-one)

### current behavior

- `DEFAULT_DONE_LIMIT = 10` - only shows 10 most recent done/skip issues
- active issues (todo/doing) unlimited, sorted by priority → created_at
- resolved issues sorted by updated_at (most recent first)
- `--all` flag bypasses the done limit

### footer counts are misleading

**issue**: the footer counts (`open: X | done: Y`) are computed AFTER truncation, so they show *displayed* counts, not *totals*. if we truncate todo issues, the footer would become even more misleading.

**fix needed**: compute totals BEFORE truncation, show them in footer.

### design proposal

**option A: separate limits for todo and doing**
```
DEFAULT_TODO_LIMIT = 15  # or 20?
# doing issues always shown (never truncated)
```

output when truncated:
```
brd-xxxx  P1  doing  current work
brd-yyyy  P2  todo   high priority
brd-zzzz  P2  todo   another one
... +12 more todo
brd-aaaa  P2  done   recent done
open: 15 (showing 3) | doing: 1 | done: 5 (showing 5) | time: 4ms
```

**option B: single limit for all active issues**
```
DEFAULT_ACTIVE_LIMIT = 20  # todo + doing combined
```

simpler, but doing issues could crowd out todos if many agents working.

### recommendation

go with **option A**:
- doing issues always visible (important to see what's in-progress)
- limit todo to 15 (enough to see priorities without overwhelming)
- fix footer to show totals: `open: 47 (showing 16)`
- indicator line: `... +32 more todo`

### questions for human

1. is 15 a good default for todo limit?
2. should the indicator be before or after the resolved issues?
3. should filters (--status, --priority) bypass the limit?

### decisions

1. **15** for default todo limit ✓
2. indicator **before** resolved issues ✓
3. filters do **not** bypass limit - use `--all` explicitly ✓
