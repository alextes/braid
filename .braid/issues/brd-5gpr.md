---
schema_version: 6
id: brd-5gpr
title: move command implementations to commands module
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-26T08:08:34.976516Z
updated_at: 2025-12-26T18:40:25.561323Z
---

extract all `cmd_*` functions from `main.rs` into a `src/commands/` module.

structure:
```
src/commands/
  mod.rs       # re-exports
  init.rs
  add.rs
  ls.rs
  show.rs
  ready.rs
  next.rs
  dep.rs
  claim.rs     # claim, release, reclaim, claims
  start.rs
  done.rs
  doctor.rs
  completions.rs
```

main.rs should only contain CLI parsing and dispatch.
