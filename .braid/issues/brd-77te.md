---
schema_version: 4
id: brd-77te
title: add brd ship command to streamline agent merge workflow
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-26T16:34:03.391298Z
updated_at: 2025-12-26T21:28:46.035129Z
acceptance:
- brd ship command exists
- refuses to run with dirty working tree (uncommitted changes)
- fetches origin main
- rebases current branch onto origin/main
- pushes current branch to main (FF only)
- on success, fetches and resets to origin/main
- on failure, leaves state intact with helpful message
---

add a `brd ship` command that handles the end of the agent workflow:

```bash
brd ship
```

should do:
1. check for clean working tree (abort if dirty)
2. `git fetch origin main`
3. `git rebase origin/main`
4. `git push origin <branch>:main` (fails if not FF)
5. on success: `git fetch origin main && git reset --hard origin/main`
6. on failure: print what went wrong and how to fix

this replaces the manual steps:
```bash
git fetch origin main && git rebase origin/main
git push origin agent-one:main
git fetch origin main && git reset --hard origin/main
```

edge cases to handle:
- dirty working tree → abort with message
- rebase conflicts → abort, tell user to resolve manually
- push rejected (not FF) → tell user main moved, rebase again
- not in a worktree → maybe warn? or just work anyway
