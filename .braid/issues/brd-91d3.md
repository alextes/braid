---
schema_version: 9
id: brd-91d3
title: add `brd path <id>` command to output issue file path
priority: P2
status: done
deps: []
tags:
- cli
owner: null
created_at: 2026-02-08T08:50:02.155482Z
started_at: 2026-02-08T08:55:32.771248Z
completed_at: 2026-02-08T09:20:54.499415Z
---

## problem

agents sometimes need to hand-edit issue files directly (e.g. to update the body, add acceptance criteria, or make edits that `brd set` doesn't cover). currently there's no straightforward way to get the file path of an issue. the path depends on configuration:

- **default mode:** `.braid/issues/<id>.md`
- **issues-branch mode:** `.git/brd/issues/.braid/issues/<id>.md`
- **external repo mode:** resolved from external repo path

agents end up guessing paths or grepping for files, which is fragile.

## existing workarounds

- `brd edit --json` outputs the path, but also requires `$EDITOR` to be set and its purpose is opening an editor, not path discovery
- the error hint in `brd edit` mentions `.braid/issues/<id>.md` but that's only the default mode path

## proposal

add a `brd path <id>` command that:

1. resolves the issue ID (supports partial IDs like other commands)
2. prints the absolute file path to stdout
3. exits with 0 on success, non-zero if issue not found

```
$ brd path q82b
/Users/alex/.braid/worktrees/braid/agent-one/.braid/issues/brd-q82b.md
```

this is the simplest, most composable approach — it works in scripts, agents, and pipe chains.

## also: update agents inject

mention `brd path <id>` in the agents inject block so agents know about it and don't resort to guessing paths.

## implementation notes

- the path computation already exists in `edit.rs` line 59: `paths.issues_dir(&config).join(format!("{}.md", full_id))`
- the command should be minimal — resolve ID, compute path, print it, done
- canonicalize the path so the output is always absolute