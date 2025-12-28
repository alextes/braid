# local-sync mode

local-sync mode keeps issues on a dedicated branch in a shared worktree, providing instant visibility between local agents.

## overview

by default (git-native mode), braid stores issues in `.braid/issues/` on the main branch. every issue claim and update syncs through git push/pull.

local-sync mode moves issue storage to a separate branch (e.g., `braid-issues`) with a shared worktree. all local agents see changes instantly without git operations.

## when to use

- multiple AI agents on the same machine
- you want instant issue coordination (no push/pull delay)
- you want to keep issue commits separate from code history
- you have frequent issue state changes

## setup

### switching to local-sync mode

```bash
brd mode sync-local              # uses default branch: braid-issues
brd mode sync-local my-issues    # custom branch name
```

this will:
1. create the sync branch from HEAD (if needed)
2. move existing issues to the sync branch
3. set up a shared worktree at `<git-dir>/brd/issues/`
4. update config with `sync_branch = "<name>"`

### new repository

you can also initialize directly in local-sync mode:

```bash
brd init --sync-branch braid-issues
```

### switching back to git-native

```bash
brd mode default
```

this copies issues back to main and removes the sync branch config.

## how it works

### directory structure

```
main branch:
  .braid/
    config.toml    # contains: sync_branch = "braid-issues"

braid-issues branch (shared worktree):
  .braid/
    config.toml
    issues/
      brd-xxxx.md
      brd-yyyy.md
```

### shared worktree

all agents share a single issues worktree at `<git-common-dir>/brd/issues/`. this directory is checked out to the sync branch.

when you run `brd start`, `brd done`, or other issue commands:
1. braid ensures the shared worktree exists
2. reads/writes issues in the shared worktree
3. changes are visible to all local agents immediately

### remote sync

use `brd sync` to push/pull with the remote:

```bash
brd sync           # commit, fetch, rebase, push
brd sync --push    # also sets upstream if not configured
```

## workflow

### claiming and completing issues

```bash
# claim an issue (instantly visible to other agents)
brd start <issue-id>

# work on it...

# mark complete (instantly visible)
brd done <issue-id>
```

### shipping code

code and issues sync separately:

```bash
# ship code to main
brd agent ship

# sync issues to remote (optional)
brd sync
```

## agent worktrees

when using local-sync mode with agent worktrees:
- each agent has their own code worktree and branch
- all agents share the single issues worktree

```bash
# from main worktree
brd agent init agent-one

# agent-one's worktree uses the shared issues worktree
cd ~/.braid/worktrees/repo/agent-one
brd start <issue-id>   # writes to shared worktree, visible to all
```

## troubleshooting

**"already in git-native mode" error:**

you're trying to switch to default mode but you're already there. check with:

```bash
brd mode
```

**"already in sync mode" error:**

switch to default first, then back to sync-local:

```bash
brd mode default
brd mode sync-local
```

**issues worktree missing:**

braid creates it automatically, but if corrupted:

```bash
rm -rf "$(git rev-parse --git-common-dir)/brd/issues"
brd ls   # triggers recreation
```

**sync conflicts:**

resolve in the issues worktree:

```bash
cd "$(git rev-parse --git-common-dir)/brd/issues"
git status
# resolve conflicts
git add .
git rebase --continue
```

## see also

- [workflow-modes.md](workflow-modes.md) — overview of all modes
- [agent-workflow.md](agent-workflow.md) — full agent worktree guide
