# Instructions for AI agents

## check, lint, test

before committing anything, or when finishing a big chunk of work, consider running:

- `cargo clippy`
- `cargo test`
- `carge fmt --all`

## commits

this repo uses [conventional commits](https://www.conventionalcommits.org/).

format: `<type>[(scope)][!]: <description>`

- `fix` → bug fix (PATCH)
- `feat` → new feature (MINOR)
- `!` or `BREAKING CHANGE:` footer → breaking change (MAJOR)
- other types: `build`, `chore`, `ci`, `docs`, `style`, `refactor`, `perf`, `test`

## writing style

all lowercase for comments, tracing, docs, and other prose. exceptions: acronyms (CLI, API, ID, etc.).
