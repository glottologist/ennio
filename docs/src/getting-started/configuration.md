# Configuration

Ennio is configured via `ennio.yaml`, discovered by searching from the current directory upward.

## Minimal Configuration

```yaml
projects:
  - name: my-project
    repo: git@github.com:user/repo.git
    path: /home/user/repos/repo
```

Everything else uses defaults: `tmux` runtime, `claude-code` agent, `worktree` workspace, port 3000.

## Environment Variable Expansion

All string values support `${VAR}` expansion:

```yaml
api_token: ${ENNIO_API_TOKEN}
projects:
  - name: app
    tracker_config:
      plugin: github
      config:
        token: ${GITHUB_TOKEN}
```

## Config Discovery

Ennio searches for `ennio.yaml` starting from the current working directory, walking up to the filesystem root. The first match wins.

## Full Reference

See the [Configuration Reference](../reference/configuration.md) for every field, its type, default value, and description.

## Per-Project Overrides

Each project can override the global defaults for runtime, agent, and workspace:

```yaml
defaults:
  runtime: tmux
  agent: claude-code
  workspace: worktree

projects:
  - name: fast-project
    runtime: process       # override: use direct process instead of tmux
    agent: aider           # override: use aider instead of claude-code
    workspace: clone       # override: full clone instead of worktree
    # ...
```

## Database

Ennio uses SQLite for persistence. The database file is created automatically:

```yaml
database_url: sqlite:ennio.db    # relative to working directory
```

If omitted, Ennio uses `sqlite::memory:` (data lost on restart).

## API Authentication

Protect the REST API with a bearer token:

```yaml
api_token: ${ENNIO_API_TOKEN}
```

All authenticated endpoints require `Authorization: Bearer <token>`. The token is compared using constant-time SHA-256 hashing to prevent timing attacks.
