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
brd ls                 # should show all issues
brd ready              # should show ready issues
```

note: `brd start` will automatically sync with origin/main before claiming.

## workflow

### 1. pick up an issue

```bash
brd ready              # see what's available
brd start <issue-id>   # syncs, claims, commits, and pushes
```

`brd start` automatically:
1. fetches and rebases on origin/main
2. marks the issue as "doing" with your agent ID
3. commits the claim
4. pushes to origin (with auto-retry on conflicts)

if push fails after retries, you'll get an error explaining the situation.

**flags:**
- `--no-sync` — skip fetch/rebase (trust local state)
- `--no-push` — claim locally without committing/pushing

### 2. work on the issue

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
brd agent merge        # rebase + push to main
```

now you're ready to pick up the next issue.

## brd agent merge

the `brd agent merge` command handles the rebase+push workflow:

1. fetches `origin main`
2. rebases your branch onto `origin/main`
3. pushes to main (fast-forward only)
4. resets your branch to `origin/main`

if main has moved and the push fails, just run `brd agent merge` again.

## how race conditions are handled

if two agents try to claim the same issue:

1. first agent to push wins
2. second agent gets a push conflict
3. second agent pulls, sees the issue already has an owner
4. second agent picks a different issue

this is optimistic locking — git handles the coordination.

## troubleshooting

**push conflict on claim:** `brd start` auto-retries twice. if it still fails, another agent likely claimed the issue. just run `brd start` again to pick a different one.

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
3. use `brd agent merge` for code changes (goes to main)

```bash
# claim and complete an issue
brd start <issue-id>
# ... work on it ...
brd done <issue-id>

# sync issues to remote
brd sync

# ship code separately
brd agent merge
```

see [sync-branch.md](sync-branch.md) for full details.

## PR-based workflow

for teams that want CI gates and human review before merging, braid supports a PR-based workflow as an alternative to `brd agent merge`.

### new commands

```bash
brd agent branch <issue-id>   # create feature branch pr/<agent>/<issue-id> from main
brd agent pr                  # create PR from current branch to main
```

### simple PR workflow (no sync branch)

```bash
brd agent branch brd-xyz   # create branch pr/agent-one/brd-xyz from main
brd start brd-xyz          # claim issue (committed in branch)
# ... do work, commit as usual ...
brd done brd-xyz           # mark done (committed in branch)
brd agent pr               # create PR
# human reviews and merges PR
# issue state merges with code
```

### PR workflow with sync branch

when using sync branch mode, issue claims are visible to all agents immediately:

```bash
brd agent branch brd-xyz   # create branch from main
brd start brd-xyz          # claim pushed to sync branch (visible to all)
# ... do work, commit as usual ...
brd agent pr               # create PR for code only
# human reviews and merges PR
brd done brd-xyz           # mark done in sync branch
brd sync                   # push done status to remote
```

### comparison

| aspect | direct workflow (`ship`) | PR workflow |
|--------|--------------------------|-------------|
| speed | fast iteration | slower (review gate) |
| audit | commit history only | PR history + reviews |
| trust | requires trust in agents | human verification |
| conflicts | resolved at ship time | resolved before merge |
| CI | optional | gates merge |

use `brd agent merge` for trusted agents working autonomously. use `brd agent pr` when you want human review or CI gates before code reaches main.
