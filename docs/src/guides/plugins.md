# Plugin System

Ennio is built around 7 pluggable slots. Each slot defines a trait, and Ennio ships with concrete implementations you can swap per-project.

## Plugin Slots

| Slot | Trait Location | Purpose |
|------|---------------|---------|
| **Agent** | `ennio-core` | The AI coding agent to run |
| **Runtime** | `ennio-core` | How the agent process is managed |
| **Workspace** | `ennio-core` | How the working directory is created |
| **Tracker** | `ennio-core` | Issue tracker integration (fetch issues, update status) |
| **SCM** | `ennio-core` | Source control (PR status, reviews, merge) |
| **Notifier** | `ennio-core` | Where notifications are sent |
| **Terminal** | `ennio-core` | Browser-based terminal access to sessions |

## Available Implementations

### Agent Plugins

| Name | Description |
|------|-------------|
| `claude-code` | Anthropic's Claude Code CLI agent |
| `aider` | Aider AI coding assistant |
| `codex` | OpenAI Codex CLI |
| `opencode` | OpenCode agent |

### Runtime Plugins

| Name | Description |
|------|-------------|
| `tmux` | Runs the agent inside a tmux session (default). Supports attach, send-keys, capture-pane. |
| `process` | Direct child process. Simpler but no terminal attach. |
| `ssh` | Runs the agent on a remote machine via SSH + tmux. |

### Workspace Plugins

| Name | Description |
|------|-------------|
| `worktree` | Creates a git worktree from the existing repo (default). Fast, shares git objects. |
| `clone` | Full shallow clone of the repository. Fully isolated but slower. |

Both workspace types support:
- `symlinks` — symlink files into the workspace (e.g., `.env`)
- `post_create` — shell commands run after workspace creation (e.g., `npm install`)

### Tracker Plugins

| Name | Description |
|------|-------------|
| `github` | GitHub Issues — fetch issue details, create branches from issue titles |
| `linear` | Linear — fetch issue details and metadata |

### SCM Plugins

| Name | Description |
|------|-------------|
| `github` | GitHub PRs — check CI status, get reviews, merge PRs, auto-merge |

### Notifier Plugins

| Name | Description |
|------|-------------|
| `desktop` | Desktop notifications via `notify-send` (Linux) or `osascript` (macOS) |
| `slack` | Slack incoming webhooks |
| `webhook` | Generic HTTP POST to any URL |

### Terminal Plugins

| Name | Description |
|------|-------------|
| `web` | WebSocket-based terminal access through the browser |

## Configuration

Set global defaults and override per-project:

```yaml
defaults:
  runtime: tmux
  agent: claude-code
  workspace: worktree
  notifiers:
    - desktop

projects:
  - name: frontend
    agent: aider            # use aider for this project
    workspace: clone        # full clone instead of worktree
    # runtime inherits tmux from defaults
```

## Agent Configuration

Fine-tune agent behavior per-project:

```yaml
projects:
  - name: my-app
    agent_config:
      model: opus
      max_turns: 200
    agent_rules:
      - "Always write tests for new code"
      - "Use conventional commits"
      - "Never modify the database schema without a migration"
```

`agent_rules` are passed to the agent as system instructions alongside the task prompt.
