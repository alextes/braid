# meta issues

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

## behavior

- `brd next` skips meta issues (they're not actionable work)
- `brd ready` includes them when all deps are done (so you can close them)
- `brd ls` shows them like regular issues

## workflow

1. create a meta issue with deps pointing to sub-tasks
2. work on the sub-tasks normally
3. when all deps are done, the meta issue appears in `brd ready`
4. close the meta issue with `brd done`
