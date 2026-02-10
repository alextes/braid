---
schema_version: 9
id: brd-ijj1
title: improve brd agent logs output with tool context and previews
priority: P2
status: done
deps: []
tags:
- agent
- cli
owner: null
created_at: 2026-02-01T19:37:48.079725Z
started_at: 2026-02-01T19:59:26.903849Z
completed_at: 2026-02-01T20:07:08.848699Z
---

## problem

current `brd agent logs` output shows unhelpful lines like:
```
  ↳ result (4204 bytes)
  ↳ result (82608 bytes)
```

this gives no indication of:
- which tool produced the result (Read, Bash, Grep, etc.)
- what the tool was doing (file path, command, pattern)
- what the result contains (preview)

## root cause

tool_result events (in `user` type messages) only contain `tool_use_id`. the formatter doesn't track preceding tool_use events to map results back to their calls.

## proposed solution

### 1. track tool_use events by ID

maintain a map of `tool_use_id -> tool_info` during log formatting:
```rust
struct ToolInfo {
    name: String,
    summary: String,  // e.g., "brd show brd-q82b" or "src/tui/ui.rs"
}
```

### 2. show tool-specific context in tool_use output

| tool | show |
|------|------|
| Bash | command (truncated) |
| Read | file path |
| Glob | pattern |
| Grep | pattern |
| Edit | file path |
| Task | subagent_type + description |
| Write | file path |

example:
```
[Bash] brd show brd-q82b
[Read] src/tui/ui.rs
[Grep] "blocking" in src/tui/
[Task:Explore] explore TUI structure
```

### 3. show tool name and preview in result

instead of `↳ result (4204 bytes)`, show:
```
  ↳ Bash: ok (4204 bytes)
  ↳ Read: ok, 150 lines
  ↳ Grep: 12 matches
  ↳ Edit: ok
  ↳ Task: completed (summary preview...)
```

for errors:
```
  ↳ Bash: error (exit 1)
  ↳ Edit: error - old_string not found
```

### 4. optional: content preview

add `--preview` flag to show first N chars/lines of result content:
```
[Bash] cargo test
  ↳ ok (4204 bytes)
    running 42 tests
    test foo::bar ... ok
    ...
```

## files to modify

- `src/commands/agent_run.rs` - `print_event()` and `format_event()`

## considerations

- keep default output concise (currently it's too sparse, but don't make it too verbose)
- maintain streaming compatibility (can't look ahead for tool_result)
- `--raw` flag should still show raw JSON