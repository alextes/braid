# braid

<img width="800" height="356" alt="braid logo, showing a terminal with the command `brd add` and a robot holding a new issue" src="https://github.com/user-attachments/assets/47b3681b-d108-400c-a299-e89fa8ee86e2" />

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/alextes/braid/check-lint-test.yaml)
![Crates.io Version](https://img.shields.io/crates/v/braid)

a lightweight, repo-local, multi-agent capable issue tracker.

**status:** ready for early users — feedback welcome! [open an issue](https://github.com/alextes/braid/issues) or reach out.

takes inspiration from [beads](https://github.com/steveyegge/beads).

## why braid?

- **issues in git** — markdown files versioned alongside code, no external service
- **dependency tracking** — issues can block other issues; `brd ready` shows what's unblocked
- **multi-agent ready** — claim-based workflow prevents duplicate work across parallel agents
- **simple cli** — partial ID matching, intuitive commands, works like you'd expect

## braid vs beads

|         | braid                               | beads                               |
| ------- | ----------------------------------- | ----------------------------------- |
| storage | markdown files in `.braid/issues/` | jsonl in `.beads/`                  |
| sync    | git-native, no daemon required      | auto-sync daemon, sqlite cache      |
| focus   | minimal cli, multi-agent workflows  | rich ecosystem, hierarchical issues |

## installation

### install script (recommended)

```bash
curl -sSL https://raw.githubusercontent.com/alextes/braid/main/install.sh | bash
```

downloads a prebuilt binary (no rust required).

### from crates.io

```bash
cargo install braid
```

requires rust 1.85+.

### prebuilt binaries

download the latest release from [GitHub Releases](https://github.com/alextes/braid/releases).

## quickstart

```bash
# initialize in your repo (follow the prompts, or use -y for defaults)
cd your-project
brd init

# create your first issue
brd add "my first task" -p P1

# start working
brd start
```

`brd init` asks two questions: where to store issues and whether to auto-sync with git. the defaults work well for most setups.

## commands

### issue management

- `brd init` — initialize braid in current repo
- `brd add "<title>" [-p P0-P3] [-b "<body>"] [--dep <id>]` — create a new issue
- `brd ls [--status todo|doing|done] [-p P0-P3] [--ready] [--blocked]` — list issues
- `brd show <id> [--context]` — show issue details (with `--context`: include deps and dependents)

### workflow

- `brd start [<id>]` — start working on an issue (auto-syncs, commits, and pushes the claim)
- `brd done <id>` — mark issue as done
- `brd ready` — list issues ready to work on

### dependencies

- `brd dep add <blocked> <blocker>` — blocked depends on blocker
- `brd dep rm <blocked> <blocker>` — remove dependency

### multi-agent

- `brd agent init <name>` — set up a new agent worktree
- `brd agent merge` — merge to main (rebase + fast-forward)

### utilities

- `brd search` — show how to search issues with grep/rg
- `brd doctor` — validate repo state
- `brd status` — show repo status summary
- `brd completions <shell>` — generate shell completions

## configuration

braid is configured via `.braid/config.toml`. key options:

- `id_prefix` — prefix for issue IDs (default: derived from repo name)
- `id_len` — length of random suffix, 4-10 (default: 4)

see [docs/configuration.md](docs/configuration.md) for full details.

## multi-agent coordination

braid enables multiple AI agents to work on the same codebase in parallel without stepping on each other's toes.

### try it now

```bash
# switch to local-sync mode (instant visibility between agents)
brd mode local-sync

# create two tasks
brd add "implement feature A"
brd add "implement feature B"

# set up two agent worktrees
brd agent init agent-one
brd agent init agent-two

# in each worktree, tell the agent to pick up work
cd .worktrees/agent-one && brd start   # claims feature A
cd .worktrees/agent-two && brd start   # claims feature B (not A!)
```

each agent automatically claims a different issue — no conflicts, no coordination needed.

### how it works

1. each agent gets their own git worktree via `brd agent init <name>`
2. when an agent runs `brd start`, the issue is marked as "doing" with their agent ID
3. in local-sync mode, all agents see claims instantly (shared filesystem)
4. in git-native mode, claims sync via git push/pull (optimistic locking)

### the workflow

```bash
# agent picks up work
brd start              # claims next ready issue

# agent does the work and commits
git add . && git commit -m "feat: implement the thing"

# agent marks done and ships
brd done <id>
brd agent merge        # rebase + fast-forward merge to main
```

## workflow modes

braid's workflow is controlled by two independent settings: **issue storage** and **auto-sync**. check your current config with `brd mode`.

| storage         | auto-sync | called        | use case              |
| --------------- | --------- | ------------- | --------------------- |
| with code       | on        | git-native    | solo, remote agents   |
| separate branch | on        | local-sync    | multiple local agents |
| external repo   | varies    | external-repo | privacy, multi-repo   |

**git-native**: issues live in `.braid/issues/` and sync via git push/pull.

**local-sync**: issues live on a separate branch in a shared worktree — all local agents see changes instantly.

**external-repo**: issues live in a separate repository entirely.

```bash
# switch to local-sync mode
brd mode local-sync

# switch to external-repo mode (external repo must exist and be initialized with brd init)
brd mode external-repo ../my-issues-repo

# switch back to git-native
brd mode git-native
```

see [docs/workflow-modes.md](docs/workflow-modes.md) for details.
