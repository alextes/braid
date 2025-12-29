# workflow modes

braid supports different workflow modes to match how you and your team work. run `brd mode` to see your current mode.

## overview

| mode | use case | setup | issue sync |
|------|----------|-------|------------|
| git-native | solo, small teams, remote agents | default | via git (push/pull main) |
| local-sync | multiple local agents | `brd mode local-sync` | instant (shared worktree) |

## git-native mode (default)

issues live alongside code in `.braid/issues/` and sync through git.

### when to use

- solo development
- small teams with occasional collaboration
- remote agents (each in their own clone)
- PR-based workflows

### how it works

1. `brd start` auto-syncs: fetches, rebases, claims, commits, and pushes
2. issue state changes flow through your normal git workflow
3. merge to main or create PRs to share issue state
4. race conditions handled by git push conflicts (optimistic locking)

### workflow

```bash
# claim an issue (auto-syncs with origin/main)
brd start

# do the work
git add . && git commit -m "feat: implement feature"

# mark done and ship
brd done <id>
git add .braid && git commit -m "done: <id>"
brd agent ship
```

### setup

this is the default mode. just initialize braid:

```bash
brd init
```

## local-sync mode

issues live on a dedicated sync branch in a shared worktree. all local agents see changes instantly.

### when to use

- multiple AI agents on the same machine
- you want instant issue visibility between agents
- you want to keep issue commits separate from code history

### how it works

1. issues live on a sync branch (e.g., `braid-issues`)
2. all local agents share a single issues worktree
3. `brd start` and `brd done` write to the shared worktree
4. changes visible instantly to all agents (shared filesystem)
5. `brd sync` pushes/pulls to remote when needed

### workflow

```bash
# claim an issue (visible to other agents immediately)
brd start

# do the work
git add . && git commit -m "feat: implement feature"

# mark done
brd done <id>

# ship code to main
brd agent ship

# optionally sync issues to remote
brd sync
```

### setup

switch from git-native to local-sync:

```bash
brd mode local-sync              # uses default branch: braid-issues
brd mode local-sync my-issues    # custom branch name
```

or initialize a new repo directly in local-sync mode:

```bash
brd init --sync-branch braid-issues
```

### switching back

to return to git-native mode:

```bash
brd mode default
```

this copies issues back to main and removes the sync branch config.

## choosing a mode

### start with git-native

git-native mode is simpler and works for most cases:
- no extra branch to manage
- familiar git workflow
- works with PRs naturally

### switch to local-sync when

- you have 2+ agents on the same machine
- agents are stepping on each other's claims
- you want cleaner main branch history

## mode-specific behavior

### `brd start`

| mode | behavior |
|------|----------|
| git-native | fetch, rebase, claim, commit, push to main |
| local-sync | claim in shared worktree (instant visibility) |

### `brd agent ship`

same in both modes: rebase + fast-forward push to main. only ships code, not issues.

### `brd sync`

| mode | behavior |
|------|----------|
| git-native | not used (issues sync via normal git) |
| local-sync | commit + push/pull sync branch |

## see also

- [agent-workflow.md](agent-workflow.md) — full agent worktree guide
- [sync-branch.md](sync-branch.md) — local-sync mode details
- [configuration.md](configuration.md) — config options
