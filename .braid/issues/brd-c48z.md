---
schema_version: 2
id: brd-c48z
title: rename brd frontmatter key to schema_version
priority: P2
status: done
deps:
- brd-heid
owner: agent-one
created_at: 2025-12-26T16:47:51.781453Z
updated_at: 2025-12-26T20:54:58.906382Z
acceptance:
- frontmatter uses schema_version instead of brd
- migration handles old brd key
---

the `brd: 1` key in issue frontmatter is unclear. rename to something self-documenting.

before:
```yaml
brd: 1
id: brd-foo
```

after:
```yaml
schema_version: 1
id: brd-foo
```

this should be the second migration after brd-2wbe sets up the migration system.
