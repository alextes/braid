# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0] - 2026-01-04

### Changed
- **BREAKING**: `brd mode` renamed to `brd config` for clearer semantics
  - `brd config` shows current settings
  - `brd config local-sync` / `brd config git-native` / `brd config external-repo` to switch modes
  - `brd config auto-sync on|off` to toggle auto-sync

### Added
- unit tests for `brd config` edge cases (clear_issues_branch, external-repo validation)

## [0.6.0] - 2026-01-04

### Added
- `brd show --context` flag to display dependents alongside dependencies
- `brd init` now errors with helpful message when braid is already initialized
- integration tests for workflow mode switching and init behavior
- unit tests for search, completions, merge, agent, and TUI modules

### Fixed
- design issue closure now prevents cycles when multiple result deps exist
- test repos now ensure branch is named "main" for CI compatibility

## [0.5.0] - 2026-01-02

### Added
- **workflow modes**: three ways to store issues
  - `git-native`: issues in `.braid/issues/` on your branch (default for solo/remote)
  - `local-sync`: issues on separate branch with shared worktree (instant local visibility)
  - `external-repo`: issues in a completely separate repository
- `brd mode` command to view and switch between workflow modes
- `brd mode local-sync [branch]` to enable local-sync mode
- `brd mode external-repo <path>` to use external repository for issues
- `brd mode git-native` to switch back to git-native mode
- `brd status` command to show repo status summary
- `brd sync` command for syncing issues in local-sync mode
- `brd agent branch <issue-id>` to create feature branch for PR workflow
- `brd agent pr` to create pull request from agent branch
- `brd agent merge` command (renamed from `ship`) with main-branch detection
- `brd start` now auto-syncs: fetch, rebase, claim, commit, push
- `brd start` warns when agent has uncompleted issues
- `brd init` interactive prompts for workflow configuration
- `brd init -y` for non-interactive setup with local-sync defaults
- design issues now require `--result <id>` when closing to link implementation
- TUI: live view mode that auto-refreshes issue list
- TUI: inline filter mode (`/` to search, status filters)
- TUI: navigate to dependency issues with enter key
- TUI: improved issue creation dialog with type selection
- schema v5: `issues_branch` field for local-sync mode
- schema v6: `auto_pull` and `auto_push` config fields
- mode-aware AGENTS.md block injection
- `#bug` tag renders in red in ls/ready output
- stats footer in ls/ready showing open/done counts

### Changed
- **BREAKING**: `brd agent ship` renamed to `brd agent merge`
- **BREAKING**: `brd dep` uses `blocker/blocked` terminology instead of `parent/child`
- init flow simplified to 2 orthogonal questions (storage + auto-sync)
- agent worktrees now created at `~/.braid/worktrees/<repo>/<agent>`
- conventional commit style for start/done issue state changes
- `brd mode` terminology: "default" renamed to "git-native"
- ls output limited to 15 todo issues with "+N more" indicator

### Fixed
- doctor prints error details inline after each failing check
- doctor includes `rm` in symlink hint for existing directories
- handle untracked changes in `brd sync`
- TUI keeps list selection visible when navigating
- priority styling separated correctly in ls/ready output
- PR branches prefixed with `pr/` to avoid git ref conflicts

## [0.4.1] - 2025-12-28

### Added
- `brd commit` command to stage and commit .braid changes with auto-generated message

### Changed
- **BREAKING**: removed control_root and dual-write pattern
  - each agent worktree now has its own `.braid/` directory
  - git is the only source of truth for issue state
  - agents sync via git pull/push (optimistic locking)
  - see updated docs/agent-workflow.md for new workflow
- agents block updated to v2 (reflects git-sync workflow)

### Removed
- `brd next` command (use `brd start` without arguments instead)

## [0.3.0] - 2025-12-27

### Added
- `brd rm` command to delete issues
- `brd skip` command to mark issues as skipped/won't do
- `brd agents` command to manage AGENTS.md instruction block
- verbose logging flag (`-v` / `BRD_VERBOSE` env var)
- TUI: issue editing with `e` key
- visual styling in `brd ls` and `brd ready`:
  - P0/P1 issues shown in bold
  - doing issues shown underlined
  - done issues shown dimmed
  - design issues shown in italic
  - meta issues shown in bold
- issue type column in ls/ready output (design/meta)
- owner shown for doing issues in ls (magenta)
- issue age in ls output (human-readable format)
- cycle prevention when adding dependencies
- configuration documentation (`docs/configuration.md`)

### Changed
- schema v4: renamed `labels` field to `tags`
- TUI priority picker now includes P0

### Fixed
- column alignment when type column is empty in ls/ready
- status column padding (doing vs todo alignment)
- doctor command AGENTS.md block status reporting

## [0.2.2] - 2025-12-27

### Added
- install script (`install.sh`) for downloading prebuilt binaries
- TUI: create issues with `a` or `n` key
- schema v3 migration: `owner` field now required

### Changed
- cargo-dist for automated release builds (linux, macos, windows)
- disabled cargo-dist's shell installer in favor of our own

### Fixed
- doctor: collapse nested if statements (clippy fix)

## [0.2.0] - 2025-12-26

### Added
- `brd agent init` command for setting up agent worktrees
- `brd agent ship` command for merging agent work to main
- `brd search` command showing how to search with grep/rg
- design issue workflow documentation
- multi-agent coordination documentation

### Changed
- refactored `cmd_add` to use `AddArgs` struct (clippy fix)
- schema v2 migration: renamed `brd` to `schema_version`

## [0.1.0] - 2025-12-20

### Added
- initial release
- `brd init` - initialize braid in a repo
- `brd add` - create issues
- `brd ls` - list issues
- `brd show` - show issue details
- `brd start` - start working on an issue
- `brd done` - mark issue as done
- `brd ready` - list ready issues
- `brd next` - show next issue to work on
- `brd dep add/rm` - manage dependencies
- TUI for browsing issues
- JSON output support

[Unreleased]: https://github.com/alextes/braid/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/alextes/braid/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/alextes/braid/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/alextes/braid/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/alextes/braid/compare/v0.3.0...v0.4.1
[0.3.0]: https://github.com/alextes/braid/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/alextes/braid/compare/v0.2.0...v0.2.2
[0.2.0]: https://github.com/alextes/braid/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/alextes/braid/releases/tag/v0.1.0
