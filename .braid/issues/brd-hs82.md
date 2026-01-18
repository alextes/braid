---
schema_version: 7
id: brd-hs82
title: design installation method for braid
priority: P1
status: done
type: design
deps: []
owner: null
created_at: 2025-12-26T08:40:04.791448Z
updated_at: 2025-12-27T08:29:15.89801Z
---

design how users will install braid. options to consider:

- cargo install from crates.io
- cargo install from git
- homebrew tap
- prebuilt binaries (github releases)
- nix flake

decide on v0.1 distribution strategy and document prerequisites.

---

## design recommendation

### v0.1 strategy

**primary: cargo install from crates.io**

```bash
cargo install braid
```

- standard rust distribution method
- automatic dependency resolution
- easy updates via `cargo install braid` again
- requires: rust toolchain (rustup)

**secondary: prebuilt binaries via github releases**

- for users without rust installed
- use `cargo-dist` to automate builds for linux/macos/windows
- single binary, no dependencies
- provide install script: `curl -sSL https://... | sh`

**keep: cargo install from git**

```bash
cargo install --git https://github.com/alextes/braid.git
```

- for bleeding edge / development versions
- already documented in README

### deferred (post v0.1)

- **homebrew tap** — nice for macos but requires maintenance
- **nix flake** — niche audience, add if requested

### cargo.toml changes needed

add these fields for crates.io publishing:

```toml
description = "a lightweight, repo-local, multi-agent capable issue tracker"
repository = "https://github.com/alextes/braid"
readme = "README.md"
keywords = ["cli", "issue-tracker", "git", "productivity"]
categories = ["command-line-utilities", "development-tools"]
```

### prerequisites to document

for cargo install:
- rust 1.85+ (edition 2024)
- git (for repo operations)

for prebuilt binaries:
- git only

### implementation tasks

1. add crates.io metadata to Cargo.toml
2. set up cargo-dist for github releases (creates release workflow)
3. update README quickstart with `cargo install braid`
4. add installation section with all methods
