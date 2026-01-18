# Getting Started with Braid

Braid is a lightweight issue tracker that stores issues as markdown files right in your git repository. No external service, no database—just files that travel with your code and sync via git.

## Installation

The quickest way to install:

```bash
curl -sSL https://raw.githubusercontent.com/alextes/braid/main/install.sh | bash
```

This downloads a prebuilt binary. Alternatively, if you have Rust installed:

```bash
cargo install braid
```

Verify the installation:

```bash
brd --version
```

## Initialize Braid in Your Repo

Navigate to your project and run:

```bash
cd your-project
brd init
```

You'll be asked two questions:
1. **Where to store issues** — press Enter for the default (with your code)
2. **Enable auto-sync** — press Enter for yes (recommended)

That's it. Braid creates a `.braid/` directory with your configuration.

## Create Your First Issue

```bash
brd add "Set up CI pipeline"
```

This creates a new issue with default priority (P2). You can set priority explicitly:

```bash
brd add "Fix critical bug" -p P0
brd add "Nice-to-have feature" -p P3
```

Priority levels: P0 (critical) → P1 (high) → P2 (normal) → P3 (low).

To add a description:

```bash
brd add "Refactor auth module" -b "Extract token validation into separate function"
```

## The Basic Workflow

### 1. See What's Ready

```bash
brd ready
```

This shows issues that are unblocked and available to work on.

### 2. Start Working

```bash
brd start
```

This picks the highest-priority ready issue and marks it as yours. You can also start a specific issue:

```bash
brd start abc1    # partial ID matching works
```

### 3. Do the Work

Make your changes and commit as usual:

```bash
git add .
git commit -m "feat: implement the thing"
```

### 4. Mark Done

```bash
brd done abc1
```

The issue moves to "done" status.

## Viewing Issues

```bash
brd ls                    # list all issues
brd ls --status doing     # filter by status
brd show abc1             # view issue details
```

## Quick Reference

| Command | What it does |
|---------|--------------|
| `brd add "title"` | Create new issue |
| `brd ls` | List issues |
| `brd ready` | Show unblocked issues |
| `brd start [id]` | Claim and start an issue |
| `brd done <id>` | Mark issue complete |
| `brd show <id>` | View issue details |

## Next Steps

- **Dependencies**: Block issues on others with `brd dep add <blocked> <blocker>`
- **Multi-agent workflows**: Set up parallel agents with `brd agent init <name>`
- **Configuration**: Customize ID prefixes, sync behavior in [configuration.md](configuration.md)
- **Workflow modes**: Learn about issues-branch and external repos in [workflow-modes.md](workflow-modes.md)

Run `brd --help` or `brd <command> --help` for full command reference.
