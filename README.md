# braid

<img width="800" height="306" alt="1b1ccb1f-c624-401a-a20e-6cc89e82a72a" src="https://github.com/user-attachments/assets/07982e37-5b97-41d8-b75a-14a0babd6c1d" />

a lightweight, repo-local, multi-agent capable issue tracker.

takes inspiration from [beads](https://github.com/steveyegge/beads).

## why braid?

- **issues as files** — markdown files with yaml frontmatter, versioned in git
- **dependency graph** — issues can depend on other issues; `brd ready` shows what's unblocked
- **deterministic selection** — `brd next` picks the highest priority ready issue, no ambiguity
- **multi-agent coordination** — multiple agents work in parallel using git worktrees
- **human-friendly** — simple CLI, partial ID matching, works like you'd expect

## quickstart

currently, compiling from source is the only installation option. more options coming soon.

```bash
# clone and install
git clone https://github.com/alextes/braid.git
cd braid
cargo install --path .

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

- `brd start [<id>]` — start working on an issue (picks next ready if no id given)
- `brd done <id>` — mark issue as done
- `brd ready` — list issues ready to work on
- `brd next` — show the next issue to work on

### dependencies

- `brd dep add <child> <parent>` — make child depend on parent
- `brd dep rm <child> <parent>` — remove dependency

### multi-agent

- `brd agent init <name>` — set up a new agent worktree

### utilities

- `brd doctor` — validate repo state
- `brd completions <shell>` — generate shell completions

## multi-agent coordination

braid enables multiple AI agents to work on the same codebase in parallel without stepping on each other's toes.

**how it works:**

1. each agent gets their own git worktree via `brd agent init <name>`
2. all worktrees share the same issue state through a shared "control root"
3. when an agent runs `brd start`, the issue is marked as "doing" with their agent ID
4. issue status changes are written to both the shared state and the agent's local branch
5. agents merge to main via rebase + fast-forward push

**the workflow:**

```bash
# agent picks up work
brd start              # claims next ready issue

# agent does the work and commits
git add . && git commit -m "feat: implement the thing"

# agent marks done and merges
brd done <id>
git add .braid/issues/<id>.md && git commit -m "chore(braid): close <id>"
git fetch origin main && git rebase origin/main
git push origin <agent-branch>:main

# agent resets for next issue
git reset --hard origin/main
```

see [docs/agent-workflow.md](docs/agent-workflow.md) for the full guide.
