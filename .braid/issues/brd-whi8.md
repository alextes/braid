---
schema_version: 7
id: brd-whi8
title: robust schema version checking across repos and modes
priority: P1
status: done
deps: []
owner: null
created_at: 2025-12-30T16:32:05.324322Z
updated_at: 2025-12-30T16:38:34.293551Z
---

Now that config has versioned migrations, we need robust schema checking to prevent old brd versions from corrupting newer repos.

## Current state

- `Config::validate()` checks `schema_version > CURRENT_SCHEMA` and errors
- Called early in main.rs after loading config
- Issue frontmatter has separate schema version (same CURRENT_SCHEMA constant)

## Gaps to address

### 1. External repo version mismatch
When using external-repo mode:
- Code repo config might be v5
- External repo config might be v6 (newer brd was used there)
- Current code doesn't check external repo's schema version before use

Should error: "external repo uses schema v6, but this brd only supports v5"

### 2. Issues worktree version mismatch
In local-sync mode:
- Main repo config: v5
- Issues worktree config: v6 (if someone ran newer brd there)
- We load resolved_config_path but may not validate it

### 3. Issue schema vs config schema
- Issues have their own schema_version in frontmatter
- Config has schema_version
- Both use CURRENT_SCHEMA but could diverge
- Should we have separate constants? CURRENT_CONFIG_SCHEMA vs CURRENT_ISSUE_SCHEMA?

### 4. Doctor should check all configs
- Main repo config
- Issues worktree config (if local-sync)
- External repo config (if external-repo)
- All issue files

### 5. Migration module awareness
- `migrate.rs` handles issue migrations
- `config.rs` now handles config migrations
- Should these be unified or at least coordinated?

## Acceptance criteria

- [ ] External repo config version is validated before use
- [ ] Issues worktree config version is validated in local-sync mode
- [ ] Doctor reports version mismatches across all configs
- [ ] Clear error messages for version mismatches
- [ ] Consider if config/issue schemas should have separate version tracks