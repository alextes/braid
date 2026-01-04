# workflow configuration

braid's workflow is controlled by two independent config dimensions. run `brd config` to see your current configuration.

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

| storage | auto-sync | use case |
|---------|-----------|----------|
| with code | on | remote agents, small teams |
| separate branch | on | multiple local agents |
| with code | off | full control, offline work |
| external repo | varies | multi-repo, privacy |

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

to get issues-with-code: answer "No" to Q1 during interactive init.

## storage configurations

### issues with code

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

### separate branch

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
brd config external-repo ../my-issues-repo
```

## changing configuration

use `brd config` to view and change settings:

```bash
brd config                              # show current settings
brd config issues-branch <name>         # enable issues branch
brd config issues-branch --clear        # disable issues branch
brd config external-repo <path>         # point to external repo
brd config external-repo --clear        # disable external repo
brd config auto-sync on|off             # enable/disable auto-sync
```

**constraints:**
- cannot have both `issues-branch` and `external-repo` set
- clear one before setting the other
- clearing `issues-branch` copies issues back to `.braid/issues/`

## auto-sync details

auto-sync works with **any** storage configuration:

| command | auto_pull | auto_push |
|---------|-----------|-----------|
| `brd start` | fetch + rebase issues | — |
| `brd done` | — | commit + push issues |

you can disable auto-sync for any storage:

```bash
brd config auto-sync off
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

- [configuration.md](configuration.md) — full config reference
