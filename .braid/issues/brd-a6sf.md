---
schema_version: 7
id: brd-a6sf
title: add started_at and completed_at, remove updated_at
priority: P2
status: open
deps: []
owner: null
created_at: 2026-01-05T21:28:54.956461Z
updated_at: 2026-01-05T21:28:54.956461Z
---

## Proposal

Replace generic `updated_at` with semantic timestamps:

- **started_at** - set when issue moves to `doing` status
- **completed_at** - set when issue moves to `done` or `skip` status

## Benefits

1. **Cycle time metrics** - completed_at - started_at = time to complete
2. **Staleness detection** - issues in `doing` for too long
3. **More meaningful** - "updated" is vague, "started/completed" tells a story

## Schema change

```yaml
# Before
created_at: 2026-01-04T12:00:00Z
updated_at: 2026-01-04T14:30:00Z

# After
created_at: 2026-01-04T12:00:00Z
started_at: 2026-01-04T13:00:00Z
completed_at: 2026-01-04T14:30:00Z
```

## Migration

- Remove `updated_at` field
- Add `started_at: null` and `completed_at: null`
- If issue is `doing`, set `started_at` to `updated_at` value
- If issue is `done`/`skip`, set both to `updated_at` value
- Bump schema version

## Considerations

- This is a breaking schema change
- Migration should preserve approximate timing from updated_at
- What if issue goes back to `todo`? Clear started_at?