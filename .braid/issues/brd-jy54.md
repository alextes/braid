---
schema_version: 9
id: brd-jy54
title: 'refactor: extract CommandContext to eliminate repeated init pattern'
priority: P2
status: skip
deps: []
tags:
- refactor
owner: null
created_at: 2026-01-27T23:29:24.204611Z
completed_at: 2026-01-28T15:31:50.602062Z
---

## problem

the same 3-line initialization pattern appears 20+ times across command files:

```rust
let config = Config::load(&paths.config_path())?;
let issues = load_all_issues(paths, &config)?;
let full_id = resolve_issue_id(id, &issues)?;
```

## proposal

create a `CommandContext` struct or helper function to consolidate this.

```rust
struct CommandContext {
    config: Config,
    issues: HashMap<String, Issue>,
}

impl CommandContext {
    fn load(paths: &RepoPaths) -> Result<Self> { ... }
    fn resolve_id(&self, partial: &str) -> Result<String> { ... }
}
```

## files affected

most files in `src/commands/` - add.rs, done.rs, start.rs, skip.rs, rm.rs, set.rs, show.rs, etc.

## conclusion: skip

after analysis, this refactor adds complexity without meaningful benefit.

**findings:**

1. there are actually 3 distinct patterns, not one:
   - config + issues + resolve (single-issue commands)
   - config + issues only (listing commands)
   - config only (config subcommands)

2. problems with CommandContext:
   - mutability: most commands need `mut issues`, requiring awkward `CommandContextMut` or interior mutability
   - unnecessary loading: config-only commands would load issues they don't need
   - lock timing: `LockGuard::acquire` currently happens explicitly between config/issue loads
   - test complexity: would add another abstraction layer on top of `TestRepo`

3. the "duplication" is only 2-3 lines per command

4. current code is explicit about what each command needs - this clarity has value

the repetition is intentional clarity, not a maintenance burden. over-applying DRY here would add abstraction for minimal benefit.
