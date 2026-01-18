---
schema_version: 7
id: brd-pwez
title: add verbose logging flag for debugging
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2025-12-26T20:35:50.61416Z
updated_at: 2025-12-27T13:38:00.369194Z
---

## context

when debugging brd behavior, it would be helpful to have verbose output showing what the tool is doing internally.

## design questions

- **flag design**: `--verbose` / `-v`? or `--debug`? support multiple levels (`-vv`)?
- **output destination**: stderr (keeps stdout clean for piping) or stdout?
- **what to log**: file operations, git commands, issue parsing, config loading?
- **implementation**: use `tracing` crate? simple eprintln? env var (`BRD_VERBOSE=1`)?
- **integration with --json**: should verbose mode work alongside JSON output?

---

## design recommendation

### flag design

**`--verbose` / `-v`** — standard convention, familiar to users.

skip multi-level verbosity (`-vv`) for now. brd is simple enough that one level suffices. can add later if needed.

also support **`BRD_VERBOSE=1`** env var for persistent debugging without modifying every command.

### output destination

**stderr** — keeps stdout clean for piping and scripting. critical for `--json` compatibility.

### what to log

keep it minimal and useful:
- repo/config discovery: "found .braid at /path/to/repo/.braid"
- config loading: "loaded config: prefix=brd, id_len=4, schema=3"
- issue operations: "loading 57 issues from .braid/issues"
- migrations: "migrating brd-xyz from v2 to v3"

avoid logging every file read — too noisy.

### implementation

**simple `eprintln!` with global flag** — brd is a small CLI, no need for `tracing` or `log` crate overhead.

```rust
// in cli.rs, add to Cli struct:
#[arg(short, long, global = true, env = "BRD_VERBOSE")]
pub verbose: bool,

// simple macro in lib.rs:
#[macro_export]
macro_rules! verbose {
    ($cli:expr, $($arg:tt)*) => {
        if $cli.verbose {
            eprintln!("[brd] {}", format!($($arg)*));
        }
    };
}
```

usage: `verbose!(cli, "loaded {} issues", count);`

### json integration

verbose logs go to stderr, json to stdout — they work together without conflict.

### implementation tasks

1. add `--verbose/-v` flag to Cli struct with `env = "BRD_VERBOSE"`
2. add `verbose!` macro
3. add verbose logging to key operations (config load, issue discovery, migrations)
4. test with `brd -v ls` and `BRD_VERBOSE=1 brd ls`
