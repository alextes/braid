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
- [docs/sync-branch.md](docs/sync-branch.md) — sync branch mode (issues on dedicated branch)
- [docs/design-issues.md](docs/design-issues.md) — design issue workflow
- [docs/release-workflow.md](docs/release-workflow.md) — how to cut releases

<!-- braid:agents:start v4 -->
## braid workflow

this repo uses braid (`brd`) for issue tracking. issues live in `.braid/issues/` as markdown files.

basic flow:
1. `brd start` — claim the next ready issue (auto-syncs, commits, and pushes)
2. do the work, commit as usual
3. `brd done <id>` — mark the issue complete
4. `brd agent ship` — push your work to main

useful commands:
- `brd ls` — list all issues
- `brd ready` — show issues with no unresolved dependencies
- `brd show <id>` — view issue details
- `brd mode` — show current workflow mode

## working in agent worktrees

**quick check — am i in a worktree?**

```bash
cat .braid/agent.toml 2>/dev/null && echo "yes, worktree" || echo "no, main"
```

if you're in a worktree:
- `brd start` handles syncing automatically
- use `brd agent ship` to merge your work to main (rebase + fast-forward push)

### schema mismatch errors

if you see an error like "this repo uses schema vX, but this brd only supports up to vY":

1. **rebase first**: `git fetch origin main && git rebase origin/main`
2. **rebuild**: `cargo build --release`
3. **if still failing**: stop and ask the human - there may be an unreleased schema change

**NEVER manually edit `schema_version` in config files.** this causes data inconsistency between agents and can corrupt the issues branch.

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

## syncing issues (local-sync mode)

this repo uses **local-sync mode** — issues live on the `braid-issues` branch in a shared worktree.

**how it works:**
- all local agents see issue changes instantly (shared filesystem)
- `brd start` and `brd done` write to the shared worktree automatically
- no manual commits needed for issue state changes

**remote sync:**
- run `brd sync` to push issue changes to the remote
- run `brd sync` to pull others' issue changes

**switching modes:**
- `brd mode` — show current mode
- `brd mode git-native` — switch to git-native mode
<!-- braid:agents:end -->
