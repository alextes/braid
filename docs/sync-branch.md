# sync branch mode

sync branch mode keeps issue tracking on a dedicated branch, separating issue commit churn from your code history on main.

## overview

by default, braid stores issues in `.braid/issues/` on the main branch. every issue claim, completion, and update creates commits on main.

sync branch mode moves all issue storage to a separate branch (e.g., `braid-issues`). code goes to main, issues go to the sync branch. this keeps your main branch history clean while still allowing issue coordination via git.

## when to use

- you have multiple agents creating frequent issue commits
- you want to keep code commits separate from issue state changes
- you prefer a cleaner main branch history

## setup

### new repository

```bash
brd init --sync-branch braid-issues
```

this creates:
1. `.braid/config.toml` on main with `sync_branch = "braid-issues"`
2. a `braid-issues` branch containing the full `.braid/` directory
3. a shared worktree at `<git-dir>/brd/issues/` for accessing issues

### existing repository

if you already have issues on main:

```bash
brd init --sync-branch braid-issues
```

this will:
1. create the `braid-issues` branch from HEAD
2. copy existing issues to the sync branch
3. update main's config to point to the sync branch
4. create the shared issues worktree

## how it works

### directory structure

```
main branch:
  .braid/
    config.toml    # contains: sync_branch = "braid-issues"

braid-issues branch:
  .braid/
    config.toml    # full config (prefix, schema, etc.)
    issues/
      brd-xxxx.md
      brd-yyyy.md
```

### shared worktree

all agents share a single issues worktree at `<git-common-dir>/brd/issues/`. this directory is checked out to the sync branch and is where all issue operations read and write.

when you run `brd start`, `brd done`, or other issue commands, braid automatically:
1. ensures the shared worktree exists
2. reads/writes issues in the worktree
3. leaves the worktree uncommitted (you push with `brd sync`)

## workflow

### claiming and completing issues

the workflow is similar to default mode, but you use `brd sync` to push issue changes:

```bash
# claim an issue
brd start <issue-id>

# work on it...

# mark complete
brd done <issue-id>

# sync issues to remote
brd sync
```

### shipping code

code and issues are now separate. use `brd agent ship` for code:

```bash
# ship code to main
brd agent ship

# sync issues to the sync branch
brd sync
```

### brd sync

the `brd sync` command handles issue synchronization:

1. stashes local issue changes
2. fetches and rebases onto `origin/<sync-branch>`
3. restores local changes
4. commits any uncommitted issue changes
5. pushes to `origin/<sync-branch>`

if there are conflicts, resolve them in the issues worktree (`<git-dir>/brd/issues/`).

## agent worktrees

when using sync branch mode with agent worktrees, each agent:
- has their own code worktree and branch (as usual)
- shares the single issues worktree with all other agents

```bash
# from main worktree
brd agent init agent-one

# agent-one's worktree will use the shared issues worktree
cd ~/.braid/worktrees/braid/agent-one
brd start <issue-id>   # writes to shared worktree
brd sync               # pushes to sync branch
```

## merging to main

the sync branch is a regular branch (not orphan), so you can merge it to main if desired:

```bash
git checkout main
git merge braid-issues
```

this brings all issue history into main. you might do this for releases or to archive the issue state.

## troubleshooting

**"not in sync branch mode" error:** your config doesn't have `sync_branch` set. initialize with:

```bash
brd init --sync-branch <branch-name>
```

**issues worktree missing:** braid creates it automatically, but if it's corrupted:

```bash
# remove and let braid recreate it
rm -rf "$(git rev-parse --git-common-dir)/brd/issues"
brd sync
```

**sync conflicts:** resolve conflicts in the issues worktree:

```bash
cd "$(git rev-parse --git-common-dir)/brd/issues"
git status
# resolve conflicts
git add .
git rebase --continue
brd sync
```

**agent can't find issues:** ensure the agent is running a recent version of brd that supports sync branch mode. check with:

```bash
brd doctor
```
