---
schema_version: 8
id: brd-jc91
title: 'design: standardize JSON output formatting'
priority: P3
status: open
type: design
deps: []
tags:
- refactor
owner: null
created_at: 2026-01-20T11:23:41.875879Z
---

## problem

inconsistent JSON output patterns across 25+ instances in 10+ files:

**pattern A - raw string literals:**
```rust
// dep.rs:47, dep.rs:74
println!(r#"{{"ok": true}}"#);
```

**pattern B - json! macro with pretty print:**
```rust
// merge.rs, sync.rs, config.rs, agent.rs
let json = serde_json::json!({
    "ok": false,
    "error": "already_on_main",
    "message": "already on main"
});
println!("{}", serde_json::to_string_pretty(&json).unwrap());
```

**pattern C - issue_to_json helper:**
```rust
// done.rs, skip.rs, set.rs, add.rs
let json = issue_to_json(&issue);
println!("{}", serde_json::to_string_pretty(&json).unwrap());
```

inconsistencies:
- error field naming varies ("ok", "error", "message" vs just "ok")
- some use raw strings, others use json! macro
- `.unwrap()` on `to_string_pretty()` inconsistent
- no standard success/error response structure

## considerations

- CLI needs consistent output for scripting/automation
- error responses should include enough info to debug
- success responses should be predictable

## possible approaches

1. **response types** - define `JsonResponse::Success(T)` and `JsonResponse::Error { code, message }`
2. **helper macros** - `json_ok!()`, `json_err!(code, msg)`, `json_issue!(issue)`
3. **output formatter** - `Output::json(value)` that handles pretty print and error handling

typed response structs would provide compile-time guarantees
