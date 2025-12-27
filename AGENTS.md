# Instructions for AI agents

## braid workflow

this repo uses braid (`brd`) for issue tracking. issues live in `.braid/issues/` as markdown files.

basic flow:
1. `brd start` — claim the next ready issue (or `brd start <id>` for a specific one)
2. do the work, commit as usual
3. `brd done <id>` — mark the issue complete

useful commands:
- `brd ls` — list all issues
- `brd ready` — show issues with no unresolved dependencies
- `brd show <id>` — view issue details

note: this repo is braid itself, so we're dogfooding it.

## working in agent worktrees

**quick check — am i in a worktree?**

```bash
cat .braid/agent.toml 2>/dev/null && echo "yes, worktree" || echo "no, main"
```

if you see `agent_id = "..."`, you're in a worktree. read [docs/agent-workflow.md](docs/agent-workflow.md) for the full workflow including:
- how issue visibility works across worktrees
- the merge workflow (rebase + fast-forward)
- how to mark issues done and push changes

## check, lint, test

before committing anything, or when finishing a big chunk of work, consider running:

- `cargo clippy`
- `cargo test`
- `cargo fmt --all`

## commits

this repo uses [conventional commits](https://www.conventionalcommits.org/).

format: `<type>[(scope)][!]: <description>`

- `fix` → bug fix (PATCH)
- `feat` → new feature (MINOR)
- `!` or `BREAKING CHANGE:` footer → breaking change (MAJOR)
- other types: `build`, `chore`, `ci`, `docs`, `style`, `refactor`, `perf`, `test`

## writing style

all lowercase for comments, tracing, docs, issue titles, and other prose. exceptions: acronyms (CLI, API, ID, etc.).

## design issues

issues with `type: design` require special handling:

- **don't close autonomously** — discuss with the human first unless the design is obvious
- **research and present options** — write up trade-offs in the issue body
- **produce output before closing** — create implementation issues or a concrete plan
- **closing = approval received** — only mark done after human signs off

see [docs/design-issues.md](docs/design-issues.md) for the full workflow.
