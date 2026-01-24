---
schema_version: 8
id: brd-r1m8
title: 'design: enforce pre-commit checks for agents'
priority: P1
status: done
type: design
deps: []
owner: null
created_at: 2026-01-24T19:16:40.011973Z
started_at: 2026-01-24T20:31:36.802871Z
completed_at: 2026-01-24T20:40:49.069403Z
---

## problem

agents sometimes create commits without running the standard checks:
- `cargo build` (or `cargo check`)
- `cargo clippy`
- `cargo fmt --all`
- `cargo test`

this leads to commits on main with lint warnings, formatting issues, or failing tests.

## options to explore

### 1. strengthen CLAUDE.md instructions
add explicit instructions in CLAUDE.md that agents MUST run checks before any commit. simple but relies on agent compliance.

### 2. git pre-commit hook via brd
`brd init` or `brd doctor` could set up a pre-commit hook that runs the checks. would catch issues at commit time.

### 3. claude code hooks
claude code supports hooks that run on certain events. could use a pre-commit or post-edit hook to remind/enforce checks.

### 4. brd commit wrapper
make `brd commit` the expected way agents commit, and have it run checks first. agents already use `brd done` which could trigger this.

## considerations
- should work for both human and agent workflows
- should fail fast with clear errors
- should be configurable (some repos may have different check commands)

---

## design analysis

### research summary

claude code has a comprehensive hooks system (documented at https://code.claude.com/docs/en/hooks):

- **PreToolUse** hooks intercept tool calls before execution
- matcher patterns like `Bash` match all bash commands
- exit code 2 blocks the action and sends stderr message to claude
- hooks configured in `.claude/settings.json` or `~/.claude/settings.json`

### option evaluation

#### 1. strengthen AGENTS.md instructions
**pros:** simple, no code changes
**cons:** relies on agent compliance, already says "consider running" which agents ignore

current text: "before committing anything, or when finishing a big chunk of work, **consider** running"
→ the word "consider" makes it optional

**verdict:** necessary but not sufficient

#### 2. git pre-commit hook
**pros:** standard mechanism, works for humans too
**cons:** agents could use `--no-verify` to bypass, slow (runs full test suite on every commit)

**verdict:** useful for humans but not reliable for agents

#### 3. claude code PreToolUse hook (recommended)
**pros:**
- intercepts at the right moment (before any `git commit`)
- can't be bypassed by agents (hook runs in claude code itself)
- error message goes back to claude so it knows what to do
- configurable per-project in `.claude/settings.json`

**cons:**
- specific to claude code (doesn't help other AI tools)
- requires a marker file mechanism to track if checks passed

**verdict:** most reliable enforcement for claude code agents

#### 4. brd commit wrapper
**pros:** could run checks automatically
**cons:** requires agents to change behavior to use `brd commit` instead of `git commit`

**verdict:** too fragile - agents will still use `git commit`

### recommendation

**hybrid approach:**

1. **claude code hook (primary enforcement)** - block `git commit` until checks pass
2. **strengthen AGENTS.md (belt)** - change "consider" to "MUST"
3. **git pre-commit hook (optional, for humans)** - run checks at commit time

### implementation details

#### claude code hook workflow

1. agent runs checks via a script: `.claude/hooks/precommit-check.sh`
2. script runs `cargo fmt --check && cargo clippy && cargo test`
3. if all pass, creates marker file: `/tmp/braid-checks-passed-{session_id}`
4. PreToolUse hook on `Bash` intercepts commands matching `git commit`
5. hook checks for marker file
   - if exists: allow commit (exit 0), delete marker
   - if missing: block (exit 2), stderr tells agent to run checks first

#### hook configuration (`.claude/settings.json`)

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "$CLAUDE_PROJECT_DIR/.claude/hooks/git-commit-guard.sh"
          }
        ]
      }
    ]
  }
}
```

#### hook script (`.claude/hooks/git-commit-guard.sh`)

```bash
#!/bin/bash
# only intercept git commit commands
INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // ""')

if [[ ! "$COMMAND" =~ ^git[[:space:]]+commit ]]; then
  exit 0  # not a commit, allow
fi

MARKER="/tmp/braid-checks-passed-$(echo "$INPUT" | jq -r '.session_id')"

if [[ -f "$MARKER" ]]; then
  rm "$MARKER"
  exit 0  # checks passed, allow commit
fi

echo "pre-commit checks required. run: cargo fmt --all && cargo clippy && cargo test" >&2
echo "after checks pass, the commit will be allowed." >&2
exit 2  # block until checks run
```

#### AGENTS.md changes

change:
```
before committing anything, or when finishing a big chunk of work, consider running:
```

to:
```
**before any commit**, you MUST run all checks and they MUST pass:

```bash
cargo fmt --all && cargo clippy && cargo test
```

commits will be blocked if checks haven't been run.
```

### alternative: simpler approach

if the marker file mechanism feels too complex, a simpler option:

**PostToolUse hook after Write/Edit** - remind agent to run checks
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "echo 'reminder: run checks before committing (cargo fmt && cargo clippy && cargo test)'"
          }
        ]
      }
    ]
  }
}
```

this is less strict but provides constant reminders.

### open questions for human review

1. should we implement the marker file approach (strict) or the reminder approach (gentle)?
2. should we also add a git pre-commit hook for human developers?
3. should check commands be configurable via `.braid/config.toml` or hardcoded in the script?