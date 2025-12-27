# issue types

braid supports optional issue types via the `type` field in frontmatter.

## meta

meta issues are tracking containers, similar to epics. they group related work and track progress via dependencies.

```yaml
---
id: brd-abc1
title: v1.0 release
type: meta
deps:
- brd-xyz1
- brd-xyz2
---
```

behavior:
- `brd next` skips meta issues (they're not actionable work)
- `brd ready` includes them when all deps are done (so you can close them)
- use deps to track sub-tasks

## design

design issues are for planning and architecture work that precedes implementation.

```yaml
---
id: brd-abc2
title: design authentication system
type: design
---
```

design issues behave like regular issues but signal that the work is planning/design rather than coding.
