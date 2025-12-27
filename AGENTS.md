# Instructions for AI agents

note: this repo is braid itself, so we're dogfooding it.

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

## releases

before cutting a release, read [docs/release-workflow.md](docs/release-workflow.md). key steps:
- review commits since last tag, update `CHANGELOG.md`
- bump version in `Cargo.toml`, run `cargo build` to update lockfile
- commit, tag, push, then `cargo publish`

## braid-specific docs

- [docs/agent-workflow.md](docs/agent-workflow.md) — full agent worktree workflow
- [docs/design-issues.md](docs/design-issues.md) — design issue workflow
- [docs/release-workflow.md](docs/release-workflow.md) — how to cut releases

**schema mismatch errors:** if you see "this repo uses schema vN, but this brd only supports up to vM", rebase onto the latest main:
```bash
git fetch origin main && git rebase origin/main && cargo build --release
```

<!-- braid:agents:start v1 -->
## braid workflow

this repo uses braid (`brd`) for issue tracking. issues live in `.braid/issues/` as markdown files.

basic flow:
1. `brd start` — claim the next ready issue (or `brd start <id>` for a specific one)
2. do the work, commit as usual
3. `brd done <id>` — mark the issue complete

useful commands:
- `brd ls` — list all issues
- `brd ready` — show issues with no unresolved dependencies
- `brd next` — show the next issue to work on
- `brd show <id>` — view issue details

## working in agent worktrees

**quick check — am i in a worktree?**

```bash
cat .braid/agent.toml 2>/dev/null && echo "yes, worktree" || echo "no, main"
```

if you're in a worktree:
- issue state is shared via the control root (usually the main repo)
- use `brd agent ship` to merge your work to main (rebase + fast-forward push)
- if you see schema mismatch errors, rebase onto latest main

## design and meta issues

**design issues** (`type: design`) require human collaboration:
- don't close autonomously — discuss with human first
- research options, write up trade-offs in the issue body
- produce output before closing (implementation issues or a plan)
- only mark done after human approves

**meta issues** (`type: meta`) are tracking issues:
- group related work under a parent issue
- show progress as "done/total" in `brd ls`
- typically not picked up directly — work on the child issues instead
<!-- braid:agents:end -->
