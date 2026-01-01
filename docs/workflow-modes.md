# workflow modes

braid supports different workflow modes to match how you and your team work. run `brd mode` to see your current mode.

## overview

| mode | use case | setup | issue sync |
|------|----------|-------|------------|
| git-native | solo, small teams, remote agents | `brd init` | via git (push/pull main) |
| local-sync | multiple local agents | `brd mode local-sync` | instant (shared worktree) |
| external-repo | separation, privacy, multi-repo | `brd mode external-repo <path>` | via external repo |

## git-native mode

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

just initialize braid:

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
brd mode git-native
```

this copies issues back to main and removes the sync branch config.

## external-repo mode

issues live in a completely separate git repository. the code repo points to the external issues repo via config.

### when to use

- separation of concerns: keep issue history separate from code history
- privacy: private issues repo with public code repo
- multi-repo coordination: one issues repo for multiple code projects

### how it works

1. you create a separate git repo for issues and initialize braid in it
2. your code repo points to the external repo via `issues_repo` config
3. all brd commands read/write from the external repo
4. the external repo can use git-native or local-sync mode internally

### workflow

```bash
# in external issues repo
cd ../my-issues-repo
git init && brd init
git add -A && git commit -m "init braid"

# in code repo, point to external issues
cd ../my-code-repo
brd mode external-repo ../my-issues-repo

# use brd normally (reads/writes to external repo)
brd add "new feature"
brd start
brd done <id>
```

### setup

1. create the external issues repo and initialize braid:

```bash
mkdir my-issues-repo && cd my-issues-repo
git init
brd init
git add -A && git commit -m "init braid"
# optionally push to remote
git remote add origin <url>
git push -u origin main
```

2. point your code repo to it:

```bash
cd my-code-repo
brd mode external-repo ../my-issues-repo
```

### switching back

to return to git-native mode:

```bash
brd mode git-native
```

note: issues remain in the external repo. you'll need to manually copy them if you want them in the code repo.

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

### switch to external-repo when

- you want issue history completely separate from code
- you need privacy (private issues, public code)
- you're coordinating issues across multiple code repos

## mode-specific behavior

### `brd start`

| mode | behavior |
|------|----------|
| git-native | fetch, rebase, claim, commit, push to main |
| local-sync | claim in shared worktree (instant visibility) |
| external-repo | claim in external repo (follows that repo's mode) |

### `brd agent ship`

same in all modes: rebase + fast-forward push to main. only ships code, not issues.

### `brd sync`

| mode | behavior |
|------|----------|
| git-native | not used (issues sync via normal git) |
| local-sync | commit + push/pull sync branch |
| external-repo | not used (sync in external repo via its mode) |

## see also

- [agent-workflow.md](agent-workflow.md) — full agent worktree guide
- [sync-branch.md](sync-branch.md) — local-sync mode details
- [configuration.md](configuration.md) — config options
