# braid

<img width="800" height="356" alt="braid logo, showing a terminal with the command `brd add` and a robot holding a new issue" src="https://github.com/user-attachments/assets/47b3681b-d108-400c-a299-e89fa8ee86e2" />

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/alextes/braid/check-lint-test.yaml)
![Crates.io Version](https://img.shields.io/crates/v/braid)

a lightweight, repo-local, multi-agent capable issue tracker.

takes inspiration from [beads](https://github.com/steveyegge/beads).

## why braid?

- **issues as files** — markdown files with yaml frontmatter, versioned in git
- **dependency graph** — issues can depend on other issues; `brd ready` shows what's unblocked
- **deterministic selection** — `brd next` picks the highest priority ready issue, no ambiguity
- **multi-agent coordination** — multiple agents work in parallel using git worktrees
- **human-friendly** — simple CLI, partial ID matching, works like you'd expect

## braid vs beads

| | braid | beads |
| --- | --- | --- |
| storage | markdown issues in `.braid/issues/` | jsonl issues in `.beads/` |
| workflow | repo-local, explicit git pull/push | distributed with auto-sync + daemon |
| ids | human-readable prefix + random suffix | hash-based ids + hierarchy |
| extras | minimal cli, no background services | sqlite cache, compaction, community tools |

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
# initialize in your repo
cd your-project
brd init

# create your first issue
brd add "my first task" -p P1

# start working
brd start
```

## commands

### issue management

- `brd init` — initialize braid in current repo
- `brd add "<title>" [-p P0-P3] [-b "<body>"] [--dep <id>]` — create a new issue
- `brd ls [--status todo|doing|done] [-p P0-P3] [--ready] [--blocked]` — list issues
- `brd show <id>` — show issue details

### workflow

- `brd start [<id>]` — start working on an issue (auto-syncs, commits, and pushes the claim)
- `brd done <id>` — mark issue as done
- `brd ready` — list issues ready to work on

### dependencies

- `brd dep add <child> <parent>` — make child depend on parent
- `brd dep rm <child> <parent>` — remove dependency

### multi-agent

- `brd agent init <name>` — set up a new agent worktree
- `brd agent ship` — push changes to main (rebase + fast-forward)

### utilities

- `brd search` — show how to search issues with grep/rg
- `brd doctor` — validate repo state
- `brd completions <shell>` — generate shell completions

## configuration

braid is configured via `.braid/config.toml`. key options:

- `id_prefix` — prefix for issue IDs (default: derived from repo name)
- `id_len` — length of random suffix, 4-10 (default: 4)

see [docs/configuration.md](docs/configuration.md) for full details.

## multi-agent coordination

braid enables multiple AI agents to work on the same codebase in parallel without stepping on each other's toes.

**getting started:** we recommend starting with a single agent in your main worktree. once you're comfortable with the braid workflow and find yourself waiting for your agent to finish, set up a second agent in its own worktree with `brd agent init`.

**model recommendations:** we've found braid works best with Claude Opus 4.5, but GPT-5.2 Codex also performs acceptably.

**how it works:**

1. each agent gets their own git worktree via `brd agent init <name>`
2. each worktree has its own `.braid/` directory — git is the source of truth
3. when an agent runs `brd start`, the issue is marked as "doing" with their agent ID
4. agents sync issue state by pulling/pushing to the sync branch (default: main)
5. race conditions are handled by git push conflicts (optimistic locking)

**the workflow:**

```bash
# agent picks up work (auto-syncs, commits, and pushes the claim)
brd start              # claims next ready issue

# agent does the work and commits
git add . && git commit -m "feat: implement the thing"

# agent marks done and ships
brd done <id>
git add .braid && git commit -m "done: <id>"
brd agent ship         # rebase + fast-forward merge to main
```

`brd start` automatically:
- fetches and rebases on origin/main
- claims the issue
- commits and pushes the claim (with auto-retry on conflicts)

use `--no-sync` to skip fetch/rebase, or `--no-push` to claim locally only.

see [docs/agent-workflow.md](docs/agent-workflow.md) for the full guide.
