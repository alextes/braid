---
brd: 1
id: brd-w83n
title: 'Design: meta/theme issues - naming and mechanics for grouping related issues'
priority: P2
status: done
deps: []
created_at: 2025-12-26T10:10:18.966011Z
updated_at: 2025-12-26T14:26:13.483448Z
---

## Problem

Sometimes we want to group related issues under a larger theme or mini-project. This is similar to "epics" but more flexible - it's about tracking a coherent set of work, not necessarily user stories.

Current state: This *works* via deps (meta issue depends on children), but lacks:
- Clear identification that an issue is a "grouping" issue
- Potentially: easy way to see progress on grouped work

## Open Questions

### Q1: What do we call them?

Options:
- A) `meta` - technical, clear
- B) `theme` - evocative, distinct from "epic"
- C) `track` - as in "tracking these issues"
- D) `umbrella` - visual metaphor
- E) `epic` - familiar but has baggage

### Q2: How to mark them?

Options:
- A) Frontmatter field: `type: meta` (or theme, etc.) - consistent with design issues
- B) Inferred: any issue with deps but no code is implicitly meta
- C) Separate field: `children: [brd-xxx, brd-yyy]` (inverse of deps)

### Q3: Is the deps mechanic sufficient?

Current: meta issue lists children as deps (blocks on them)
- Pro: Already works, no new mechanics
- Con: Semantically backwards? Meta doesn't "depend on" children, it "contains" them
- Con: deps field gets cluttered if meta has many children

Alternative: Add `children` or `contains` field (inverse relationship)

### Q4: Display treatment?

- Show progress? (e.g., "3/5 done")
- Visual distinction in `brd ls`?
- Hierarchical display option?

## Decisions

- **Q1**: Call them `meta`
- **Q2**: Mark with `type: meta` (consistent with `type: design`)
- **Q3**: Keep deps mechanic for now - semantically "done when children done" works
- **Q4**: Progress indicator and visual distinction as separate issues; defer tree display

## Spawned Issues

- brd-zxho: Display meta-type issues with visual distinction in brd ls
- brd-6h0b: Show progress indicator for meta issues (e.g. 3/5 done)

