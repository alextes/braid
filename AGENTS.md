# Instructions for AI agents

note: this repo is braid itself, so we're dogfooding it.

## check, lint, test

**before any commit**, you MUST run all checks and they MUST pass:

```bash
cargo fmt --all && cargo clippy && cargo test
```

do not commit if any of these fail. fix issues first, then commit.

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

- [docs/sync-branch.md](docs/sync-branch.md) — sync branch mode (issues on dedicated branch)
- [docs/design-issues.md](docs/design-issues.md) — design issue workflow
- [docs/release-workflow.md](docs/release-workflow.md) — how to cut releases

<!-- braid:agents:start v8 -->
## braid workflow

this repo uses braid (`brd`) for issue tracking. issues live in `.braid/issues/` as markdown files.

basic flow:
1. `brd start` — claim the next ready issue
2. do the work, commit as usual
3. `brd done <id>` — mark the issue complete
4. ship your work:
   - in a worktree: `brd agent merge` (rebase + ff-merge to main)
   - on main: just `git push` (you're already there)

useful commands:
- `brd ls` — list all issues
- `brd ready` — show issues with no unresolved dependencies
- `brd show <id>` — view issue details (shows deps and dependents)
- `brd show <id> --context` — view issue with full content of related issues
- `brd config` — show current workflow configuration
- `brd path <id>` — print the absolute file path of an issue (for hand-editing)

**tip:** before starting work, use `brd show <id> --context` to see the issue plus all its dependencies and dependents in one view.

## working on main vs in a worktree

**quick check — am i in a worktree?**

```bash
cat .braid/agent.toml 2>/dev/null && echo "yes, worktree" || echo "no, main"
```

**if you're in a worktree (feature branch):**
- `brd start` handles syncing automatically
- use `brd agent merge` to ship (rebase + ff-merge to main)
- if you see schema mismatch errors, rebase onto latest main

**if you're on main:**
- `brd start` syncs and claims
- after `brd done`, just `git push` your code commits
- no `brd agent merge` needed — you're already on main

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

## syncing issues (issues-branch mode)

this repo uses **issues-branch** — issues live on the `braid-issues` branch in a shared worktree.

**how it works:**
- all local agents see issue changes instantly (shared filesystem)
- `brd start` and `brd done` write to the shared worktree automatically
- no manual commits needed for issue state changes

**remote sync:**
- run `brd sync` to push issue changes to the remote
- run `brd sync` to pull others' issue changes

**changing settings:**
- `brd config` — show current config
- `brd config issues-branch --clear` — disable issues-branch
<!-- braid:agents:end -->
