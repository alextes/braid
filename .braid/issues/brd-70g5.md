---
schema_version: 9
id: brd-70g5
title: 'design: rename brd dep to use blocking language'
priority: P2
status: done
type: design
deps: []
tags:
- ux
owner: null
created_at: 2026-01-18T20:42:58.628755Z
completed_at: 2026-02-07T19:14:56.558824Z
---

The codebase shifted from "dependency" to "blocking/blocked" language because it's clearer:
- "X blocks Y" / "Y is blocked by X" is intuitive
- "parent is transitive dep of child" (design→impl issues) is confusing

But the command is still `brd dep`. Should we rename it?

## current state

- command: `brd dep add <blocked> <blocker>`
- params already use blocking language (good!)
- output: "added dependency: X blocked by Y" (mixed)
- field in issues: `deps: []`
- show output: `Deps:`, `Dependents:`

## options

### 1. rename command to `brd block`
```
brd block add <issue> <blocker>
brd block rm <issue> <blocker>
```
- alias `brd dep` for backwards compat?
- clear intent

### 2. alternative: `brd blocks` (verb form)
```
brd blocks add <blocker> <blocked>  # "X blocks Y"
```
- note: reversed argument order
- reads naturally as sentence

### 3. keep `brd dep`, update messaging only
- change "added dependency" → "added blocker"
- update help text
- least disruptive

### 4. rename field `deps` to `blocked_by`?
- requires migration
- more explicit but longer
- separate concern from command naming

## questions
- is backwards compat important? (alias vs hard rename)
- should `brd show` output change too? (`Deps:` → `Blocked by:`)