# configuration

braid stores its configuration in `.braid/config.toml`. this file is created by `brd init` and can be edited manually.

## example

```toml
schema_version = 5
id_prefix = "brd"
id_len = 4
```

## options

### id_prefix

the prefix used for issue IDs.

- **type:** string
- **default:** derived from repo directory name (first 4 alphanumeric chars)
- **constraints:** 2-12 characters

when you run `brd init` in a directory called `my-project`, the prefix defaults to `mypr`. issues will be created as `mypr-a1b2`, `mypr-c3d4`, etc.

you can change this to any short identifier that makes sense for your project:

```toml
id_prefix = "acme"    # acme-a1b2
id_prefix = "web"     # web-c3d4
id_prefix = "api"     # api-e5f6
```

### id_len

the length of the random suffix in issue IDs.

- **type:** integer
- **default:** 4
- **constraints:** 4-10

with `id_len = 4`, you get IDs like `brd-a1b2`. increase this if you have many issues and start seeing collisions (braid retries up to 20 times before giving up).

```toml
id_len = 6    # brd-a1b2c3
```

### issues_branch

when set, issues live on this branch in a shared worktree instead of alongside code.

- **type:** string (optional)
- **default:** not set (issues live in `.braid/issues/`)

set via `brd config issues-branch <name>`, clear via `brd config issues-branch --clear`. see [workflow-modes.md](workflow-modes.md) for details.

```toml
issues_branch = "braid-issues"
```

### issues_repo

when set, issues are read from and written to this external repository.

- **type:** string (optional)
- **default:** not set
- **value:** path to external repo (relative or absolute)

set via `brd config external-repo <path>`, clear via `brd config external-repo --clear`. see [workflow-modes.md](workflow-modes.md) for details.

```toml
issues_repo = "../my-issues-repo"
```

### auto_pull / auto_push

control automatic git sync on `brd start` and `brd done`.

- **type:** boolean
- **default:** true

set via `brd config auto-sync on|off` (sets both together).

```toml
auto_pull = true   # fetch + rebase before brd start
auto_push = true   # commit + push after brd done
```

### schema_version

internal version number for the issue schema. **do not edit this manually** - it's managed by braid and used for migrations.

if you see an error like "this repo uses schema vX, but this brd only supports up to vY", you need to upgrade braid.
