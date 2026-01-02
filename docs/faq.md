# FAQ

## How do I browse issues in my editor when using an issues branch?

When using local-sync mode (`issues_branch` configured), issues live on a separate branch in a worktree at `.git/brd/issues/`. Your main editor window won't see them directly.

**Recommended**: Open a second editor window on the issues worktree:
```sh
code .git/brd/issues    # vscode
nvim .git/brd/issues    # neovim
```

**Alternative**: Create a symlink in your working directory:
```sh
ln -s .git/brd/issues/.braid/issues .issues
```

Then browse `.issues/` from your main editor. Note: don't commit the symlink.
