# release workflow

how to cut a new braid release.

## 1. review changes since last release

```bash
git log --oneline $(git describe --tags --abbrev=0)..HEAD
```

## 2. update changelog

edit `CHANGELOG.md`:
- move items from `[Unreleased]` to a new version section
- add the release date
- update the comparison links at the bottom

```markdown
## [Unreleased]

## [0.3.0] - 2025-12-28

### Added
- ...
```

## 3. bump version

update `version` in `Cargo.toml`:

```toml
version = "0.3.0"
```

## 4. update lockfile

run a build so `Cargo.lock` picks up the new version:

```bash
cargo build
```

## 5. commit the release

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "release: v0.3.0"
```

## 6. ship to main (if in agent worktree)

```bash
brd agent ship
```

## 7. tag and push

```bash
git tag v0.3.0
git push --tags
```

this triggers cargo-dist CI which builds binaries for all platforms and creates the github release.

## 8. publish to crates.io

after CI passes:

```bash
git checkout v0.3.0
cargo publish
```

## pre-flight checks

before releasing, ensure:

```bash
cargo check
cargo clippy
cargo fmt --all -- --check
cargo test
```

all must pass — CI will reject the release otherwise.
