# agent worktree workflow

this document describes the workflow for AI agents working in git worktrees with braid.

## overview

braid supports multiple agents working in parallel using git worktrees. each agent gets their own worktree with a dedicated branch, while sharing the same issue state.

key concepts:

- **control root**: the main worktree that owns the canonical `.braid/issues/` directory
- **dual-write**: when agents modify issue status, changes go to both control root (for immediate visibility) and the local worktree (for git commits)

## setup

### creating an agent worktree

from the main worktree:

```bash
brd agent init <agent-name>
```

this creates:

1. a git worktree at `../<agent-name>` with branch `<agent-name>`
2. `.braid/agent.toml` with `agent_id = "<agent-name>"`

### verifying setup

from your agent worktree:

```bash
brd ls          # should show all issues
brd ready       # should show ready issues
```

## workflow

### 1. pick up an issue

```bash
brd ready              # see what's available
brd next               # get highest priority ready issue
brd start <issue-id>   # marks as "doing", sets you as owner
```

`brd start` writes to both control root and your local worktree, so you can commit the status change with your code.

### 2. work on the issue

make your changes and commit:

```bash
git add .
git commit -m "feat: implement the thing"
```

the issue status change (from `brd start`) is included in your commit automatically.

### 3. mark done

when finished:

```bash
brd done <issue-id>
git add .braid/issues/<issue-id>.md
git commit -m "chore(braid): close <issue-id>"
```

### 4. ship to main

use `brd agent ship` to push your changes to main:

```bash
brd agent ship
```

this command:
1. fetches `origin main`
2. rebases your branch onto `origin/main`
3. pushes to main (fast-forward only)
4. resets your branch to `origin/main`

if main has moved and the push fails, just run `brd agent ship` again.

now you're ready to pick up the next issue.

## how it works

### issue visibility

all worktrees share the same issue state via the control root mechanism:

```
git common dir:  /path/to/repo/.git
control root:    /path/to/repo  (main worktree)
```

when you run `brd ls` from any worktree, it reads from the control root's `.braid/issues/`.

### dual-write

when you run `brd start` or `brd done` from a non-control-root worktree:

1. writes to control root (other agents see the change immediately)
2. writes to your local `.braid/issues/` (so you can commit it)

this way, issue status changes flow through git like code changes.
