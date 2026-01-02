# workflow configuration

braid's workflow is controlled by two independent config dimensions. run `brd mode` to see your current configuration.

## two dimensions

### 1. issue storage

where do issues live?

| setting | location | visibility |
|---------|----------|------------|
| `issues_branch = None` | `.braid/issues/` on your branch | issues travel with code |
| `issues_branch = "braid-issues"` | shared worktree | instant local visibility |
| `issues_repo = "../path"` | external repository | fully separate |

### 2. auto-sync

should braid sync with git automatically?

| setting | behavior |
|---------|----------|
| `auto_pull = true` | fetch + rebase before `brd start` |
| `auto_push = true` | commit + push after `brd done` |
| both `false` | manual sync only (`brd sync`) |

these are **independent** — you can combine any storage with any sync setting.

## common configurations

| storage | auto-sync | called | use case |
|---------|-----------|--------|----------|
| with code | on | git-native | remote agents, small teams |
| separate branch | on | local-sync | multiple local agents |
| with code | off | manual | full control, offline work |
| external repo | varies | external-repo | multi-repo, privacy |

## brd init

`brd init` asks two questions to configure your workflow:

```
Q1: Use a separate branch for issues? [recommended]
    1. Yes → issues_branch = "braid-issues"
    2. No  → issues_branch = None (issues with code)

Q2: Auto-sync with git remote?
    1. Yes → auto_pull = true, auto_push = true
    2. No  → manual sync only
```

**defaults:**
- `brd init` (interactive): asks both questions
- `brd init -y` (non-interactive): `issues_branch = "braid-issues"`, auto-sync enabled

to get issues-with-code (git-native): answer "No" to Q1 during interactive init.

## storage configurations

### issues with code (git-native)

issues live in `.braid/issues/` alongside your code.

```toml
# .braid/config.toml
issues_branch = # not set
auto_pull = true
auto_push = true
```

**how it works:**
- `brd start` fetches, rebases, claims issue, commits, pushes
- issue changes flow through normal git workflow
- race conditions handled by git push conflicts

**workflow:**
```bash
brd start              # claim issue (syncs with remote)
# work, commit
brd done <id>          # mark done (pushes to remote)
```

### separate branch (local-sync)

issues live on a dedicated branch in a shared worktree.

```toml
# .braid/config.toml
issues_branch = "braid-issues"
auto_pull = true
auto_push = true
```

**how it works:**
- issues stored at `.git/brd/issues/` (shared worktree)
- all local agents see changes instantly (shared filesystem)
- `brd sync` pushes/pulls the issues branch to remote

**workflow:**
```bash
brd start              # claim issue (instant local visibility)
# work, commit
brd done <id>          # mark done (instant local visibility)
brd sync               # push issues to remote when ready
```

### external repository

issues live in a completely separate git repo.

```toml
# .braid/config.toml
issues_repo = "../my-issues-repo"
```

**how it works:**
- external repo has its own braid config
- all brd commands read/write to external repo
- external repo can use any storage mode internally

**setup:**
```bash
# create external issues repo
mkdir ../my-issues-repo && cd ../my-issues-repo
git init && brd init -y
git add -A && git commit -m "init braid"

# point code repo to it
cd ../my-code-repo
brd mode external-repo ../my-issues-repo
```

## switching configurations

use `brd mode` to change your configuration:

```bash
brd mode                           # show current config
brd mode local-sync                # enable issues_branch
brd mode local-sync my-branch      # custom branch name
brd mode external-repo ../path     # point to external repo
brd mode git-native                # clear issues_branch/issues_repo
```

**constraints:**
- switching to local-sync or external-repo requires being in git-native first
- to switch between local-sync and external-repo: go through git-native
- `brd mode git-native` copies issues back to `.braid/issues/` if needed

## auto-sync details

auto-sync works with **any** storage configuration:

| command | auto_pull | auto_push |
|---------|-----------|-----------|
| `brd start` | fetch + rebase issues | — |
| `brd done` | — | commit + push issues |

you can disable auto-sync for any storage mode:

```toml
# local-sync with manual remote sync
issues_branch = "braid-issues"
auto_pull = false
auto_push = false
```

then use `brd sync` when you want to share with remote.

## config reference

```toml
# .braid/config.toml
schema_version = 6
id_prefix = "brd"
id_len = 4

# storage (pick one)
issues_branch = "braid-issues"   # or omit for issues-with-code
# issues_repo = "../path"        # or point to external repo

# sync behavior
auto_pull = true                 # fetch+rebase on brd start
auto_push = true                 # commit+push on brd done
```

## see also

- [agent-workflow.md](agent-workflow.md) — agent worktree guide
- [configuration.md](configuration.md) — full config reference
