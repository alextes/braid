# braid

<img width="800" height="306" alt="1b1ccb1f-c624-401a-a20e-6cc89e82a72a" src="https://github.com/user-attachments/assets/07982e37-5b97-41d8-b75a-14a0babd6c1d" />

a lightweight, repo-local issue tracker built for parallel agent workflows.

takes inspiration from [beads](https://github.com/steveyegge/beads), with a focus on multi-agent coordination.

## why braid?

- **issues as files** — markdown files with yaml frontmatter, versioned in git
- **dependency graph** — issues can depend on other issues; `brd ready` shows what's unblocked
- **deterministic selection** — `brd next` picks the highest priority ready issue, no ambiguity
- **multi-agent coordination** — multiple agents work in parallel using git worktrees, with claims preventing collisions
- **human-friendly** — simple CLI, partial ID matching, works like you'd expect
