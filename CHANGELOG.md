# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/alextes/braid/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/alextes/braid/compare/v0.2.0...v0.2.2
[0.2.0]: https://github.com/alextes/braid/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/alextes/braid/releases/tag/v0.1.0
