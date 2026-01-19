---
schema_version: 8
id: brd-664s
title: 'design: bidirectional dependency exploration for agents'
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2026-01-03T12:57:36.265137Z
started_at: 2026-01-04T13:39:58.938932Z
completed_at: 2026-01-04T13:39:58.938932Z
---

deps are useful not only to indicate the sequence in which issues should be picked up, they help users of brd explore which issues are related - open or closed. this is especially useful for agents which may be starting work on an issue beginning from zero context, unlike humans which tend to automatically remember context both consciously and subconsciously.

however, deps currently only easy to explore in one direction. when viewing an issue, you can see what it depends on, but not what depends on it.

## problem

given issue A:
- A depends on B, C (easy to see in frontmatter: `deps: [B, C]`)
- D, E depend on A (not visible when viewing A)

an agent starting work on A would benefit from knowing about D and E:
- they provide context about why A matters
- they may contain requirements or constraints
- completing A may unblock them

## questions

1. where should dependents be shown?
   - in `brd show` output?
   - in the issue file itself (computed field)?
   - both?

2. should this be opt-in or always visible?

3. for agents specifically, should `brd show` have a mode that outputs all related context?

## options to explore

### option 1: show dependents in `brd show`
add a "Blocked by this:" or "Dependents:" section to show output

### option 2: computed frontmatter field
when reading an issue, compute and display dependents inline

### option 3: `brd show --context` or `brd show --full`
expanded output mode that shows deps, dependents, and possibly related issue content

### option 4: `brd related <id>`
separate command to explore the dependency graph around an issue

## considerations

- performance: computing dependents requires scanning all issues
- caching: could cache the reverse-dep graph
- agents vs humans: agents may want more verbose output than humans

---

## implementation

implemented option 1 + option 3:

### phase 1: always show dependents in `brd show`
- added `get_dependents()` to `src/graph.rs` - computes reverse deps by scanning all issues
- updated `issue_to_json()` to include `"dependents"` field
- added `Dependents:` line to text output in `format_show_output()`

### phase 2: `--context` flag for deep exploration
- added `--context` flag to show command
- `brd show <id> --context` outputs:
  1. main issue header and full content
  2. `=== Dependencies ===` section with each dep's status and body
  3. `=== Dependents ===` section with each dependent's status and body

gives agents complete context about an issue in a single command.

### files changed
- `src/graph.rs` - `get_dependents()` function
- `src/commands/mod.rs` - updated `issue_to_json()`
- `src/commands/show.rs` - dependents output, `format_context_output()`
- `src/cli.rs` - `--context` flag
- `src/main.rs` - pass context flag to cmd_show