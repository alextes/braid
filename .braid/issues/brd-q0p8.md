---
schema_version: 2
id: brd-q0p8
title: 'Design: design/stub issues - issues that need fleshing out before coding'
priority: P2
status: done
deps: []
created_at: 2025-12-26T10:10:19.062158Z
updated_at: 2025-12-26T14:16:29.900144Z
---

## Problem

Some issues can't be coded yet - they need thinking first. These "design stubs" have distinct characteristics:

1. **Not code-ready** - The work is figuring out *what* to build, not building it
2. **Collaborative** - Often needs human + agent iteration with high thinking
3. **Produces artifacts** - Results in either expanded spec or spawned child issues

## Open Questions

### Q1: How to mark an issue as design-type?

Options:
- A) Frontmatter field: `type: design`
- B) Status value: `status: design` (before `todo`)
- C) Convention: title prefix like "Design:" or "RFC:"
- D) Separate file location or extension

### Q2: What's the lifecycle?

Options:
- A) Design issue expands in-place → eventually becomes `todo` → worked on
- B) Design issue spawns children → closes as `done` when design complete
- C) Design issue spawns children → new status like `resolved` or `superseded`

### Q3: Should `brd ls` treat design issues specially?

- Exclude from default list? (require `brd ls --design` or `brd ls --all`)
- Show but visually distinguish?
- Show inline with others?

## Decisions

- **Q1**: Frontmatter field `type: design`
- **Q2**: Design issues spawn children → close as `done` when design complete
- **Q3**: Show in default `brd ls`, visually distinguish (like done issues)

## Spawned Issues

- brd-20qi: Add type field support to issue frontmatter
- brd-d42n: Display design-type issues with visual distinction in brd ls

