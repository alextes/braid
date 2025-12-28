# agent worktree workflow

this document describes the workflow for AI agents working in git worktrees with braid.

## overview

braid supports multiple agents working in parallel using git worktrees. each agent gets their own worktree with a dedicated branch and their own `.braid/` directory.

key concepts:

- **git as source of truth**: issue state is synchronized via git pull/push
- **issue files are claims**: `status: doing` + `owner: agent-id` = claimed
- **optimistic locking**: git push conflicts prevent duplicate claims

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
git pull origin main   # sync latest issue state
brd ls                 # should show all issues
brd ready              # should show ready issues
```

## workflow

### 1. sync and pick up an issue

```bash
git pull origin main   # get latest issue state
brd ready              # see what's available
brd start <issue-id>   # marks as "doing", sets you as owner
```

### 2. commit the claim

push your claim so other agents see it:

```bash
git add .braid
git commit -m "start: <issue-id>"
git push origin main
```

if the push fails (another agent pushed first), pull and check if the issue is still available:

```bash
git pull --rebase origin main
brd show <issue-id>    # check if someone else claimed it
```

### 3. work on the issue

make your changes and commit:

```bash
git add .
git commit -m "feat: implement the thing"
```

### 4. mark done and ship

when finished:

```bash
brd done <issue-id>
git add .braid
git commit -m "done: <issue-id>"
brd agent ship         # rebase + push to main
```

now you're ready to pick up the next issue.

## brd agent ship

the `brd agent ship` command handles the rebase+push workflow:

1. fetches `origin main`
2. rebases your branch onto `origin/main`
3. pushes to main (fast-forward only)
4. resets your branch to `origin/main`

if main has moved and the push fails, just run `brd agent ship` again.

## how race conditions are handled

if two agents try to claim the same issue:

1. first agent to push wins
2. second agent gets a push conflict
3. second agent pulls, sees the issue already has an owner
4. second agent picks a different issue

this is optimistic locking — git handles the coordination.

## troubleshooting

**push conflict on claim:** another agent claimed the issue first. pull and pick a different issue:

```bash
git pull --rebase origin main
brd ready              # pick another issue
```

**schema mismatch errors:** if you see "this repo uses schema vN, but this brd only supports up to vM", rebase onto the latest main:

```bash
git fetch origin main && git rebase origin/main && cargo build --release
```

**stale claim:** if an issue is stuck as "doing" but the agent crashed, the owner can reclaim it:

```bash
brd start <issue-id>   # if you're the owner, you can restart it
```

## sync branch mode

if your repository uses sync branch mode, issues live on a separate branch instead of main. this keeps issue commit churn separate from code history.

with sync branch mode enabled:

1. issue commands (`brd start`, `brd done`, etc.) write to a shared issues worktree
2. use `brd sync` to push issue changes to the sync branch
3. use `brd agent ship` for code changes (goes to main)

```bash
# claim and complete an issue
brd start <issue-id>
# ... work on it ...
brd done <issue-id>

# sync issues to remote
brd sync

# ship code separately
brd agent ship
```

see [sync-branch.md](sync-branch.md) for full details.
