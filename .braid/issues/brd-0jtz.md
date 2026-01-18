---
schema_version: 7
id: brd-0jtz
title: require result issues when closing design issues
priority: P1
status: done
deps: []
owner: null
created_at: 2025-12-28T21:33:58.805982Z
updated_at: 2025-12-28T21:39:11.440357Z
---

when marking a design issue as done, require specifying which issues were created as a result.

## behavior

```bash
brd done <design-id> --result <issue-id> [--result <issue-id>...]
```

- fails if referenced issue IDs don't exist
- `--force` flag to close anyway (for edge cases)
- design issues without `--result` (and without `--force`) should error with helpful message

## transitive deps

if issue X depends on a design issue D, and D is closed with `--result A, B`:
- X should transitively depend on A and B
- this allows declaring design issues as deps before knowing what work will result

## docs

update `docs/design-issues.md` to document this behavior.

## rationale

prevents agents from accidentally closing design issues without creating follow-up work. enforces the design workflow at the tool level.