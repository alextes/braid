# release workflow

how to cut a new braid release.

## pre-flight checks

before releasing, run locally with CI-equivalent strictness:

```bash
cargo check
cargo clippy -- -D warnings
cargo fmt --all -- --check
cargo test --all
```

all must pass — CI will reject the release otherwise.

## 1. review changes since last release

```bash
git log --oneline $(git describe --tags --abbrev=0)..HEAD
```

## 2. update changelog

edit `CHANGELOG.md`:

- move items from `[Unreleased]` to a new version section
- update the comparison links at the bottom

```markdown
## [Unreleased]

## [0.4.0]

### Added

- ...
```

## 3. bump version

update `version` in `Cargo.toml`:

```toml
version = "0.4.0"
```

## 4. update lockfile

run a build so `Cargo.lock` picks up the new version:

```bash
cargo build
```

## 5. commit the release

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "release: v0.4.0"
```

## 6. ship to main (if in agent worktree)

```bash
brd agent merge
```

## 7. push and verify CI

push commits to main and **wait for CI to pass** before tagging:

```bash
git push origin main
```

check CI status with `hub ci-status -v` or at https://github.com/alextes/braid/actions — do not proceed until all checks pass.

## 8. tag and push

only after CI passes:

```bash
git tag v0.4.0
git push --tags
```

this triggers cargo-dist CI which builds binaries for all platforms and creates the github release.

## 9. publish to crates.io

after cargo-dist CI passes:

```bash
git checkout v0.4.0
cargo publish
```
