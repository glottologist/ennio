# Ennio

An AI agent orchestrator, named after [Ennio Morricone](https://en.wikipedia.org/wiki/Ennio_Morricone) — the legendary Italian composer who conducted entire orchestras to create something greater than any single instrument could achieve. Ennio does the same with AI coding agents: it orchestrates multiple agents across repositories, monitors their progress through the full PR lifecycle, and reacts when things go wrong.

## What It Does

Ennio spawns AI coding agents (Claude Code, Aider, Codex, OpenCode) in isolated workspaces, monitors their session lifecycle from first commit through merged PR, and automatically reacts to CI failures, review feedback, and merge conflicts. It works both locally and on remote machines over SSH.

### Session Lifecycle

Each agent session progresses through a state machine:

```
Spawning → Working → PR Draft → PR Open → CI Passing → Review Pending → Approved → Merged → Done
                                    ↓           ↓              ↓
                                CI Failed   Changes Requested  Merge Conflicts
                                    ↓           ↓              ↓
                              CI Fix Sent   (agent fixes)   (agent rebases)
```

Ennio polls external state (CI status, PR reviews, comments) and triggers configurable reactions — sending instructions back to the agent, notifying you, or auto-merging approved PRs.

## Installation

### Nix (recommended)

```bash
# Build the CLI
nix build github:glottologist/ennio

# Build the remote node daemon
nix build github:glottologist/ennio#ennio-node

# Run directly
nix run github:glottologist/ennio

# Enter dev shell
nix develop github:glottologist/ennio
```

From a local checkout:

```bash
nix build            # builds ennio CLI (default)
nix build .#ennio-node
nix build .#ennio-dashboard
nix run              # run the CLI
nix flake check      # clippy, fmt, tests, docs, audit
nix develop          # dev shell with cargo-nextest, rust-analyzer, bacon, etc.
```

### Cargo

```bash
cargo build --release -p ennio-cli -p ennio-node
```

Binaries are at `target/release/ennio` and `target/release/ennio-node`.

### Docker

```bash
docker build -t ennio .
docker run --rm ennio --help
docker run --rm --entrypoint ennio-node ennio --help
```

Pre-built images are published to `glottologist/ennio` on Docker Hub for tagged releases.

## Quick Start

### 1. Initialize a config

```bash
ennio init .
```

This creates `ennio.yaml` with sensible defaults.

### 2. Configure a project

```yaml
port: 3000
defaults:
  runtime: tmux
  agent: claude-code
  workspace: worktree

projects:
  - name: my-project
    repo: git@github.com:user/repo.git
    path: /home/user/repos/my-project
    default_branch: main
    tracker_config:
      provider: github
      owner: user
      repo: repo
      token: ${GITHUB_TOKEN}
    scm_config:
      provider: github
      owner: user
      repo: repo
      token: ${GITHUB_TOKEN}
    post_create:
      - npm install
    agent_rules:
      - "Always write tests for new code"
      - "Follow existing code style"
```

### 3. Start the orchestrator

```bash
ennio start
```

### 4. Spawn an agent session

```bash
# Work on a GitHub issue
ennio spawn my-project --issue 42

# Work on a direct prompt
ennio spawn my-project --prompt "Add user authentication"

# Specify a branch name
ennio spawn my-project --issue 42 --branch feat/auth
```

### 5. Monitor sessions

```bash
# List all sessions
ennio status

# Filter by project
ennio status my-project

# Session details
ennio session info <session-id>

# Open the web dashboard
ennio dashboard

# Open TUI dashboard
# (available via the ennio-tui library)
```

### 6. Interact with sessions

```bash
# Send a message to a running agent
ennio send <session-id> "Also add input validation"

# Open the agent's terminal
ennio open <session-id>

# Kill a session
ennio session kill <session-id>

# Restore an exited session
ennio session restore <session-id>
```

## Plugin System

Ennio is built around 7 pluggable slots. Each project can override the defaults.

| Slot | Implementations | Description |
|------|----------------|-------------|
| **Agent** | `claude-code`, `aider`, `codex`, `opencode` | AI coding agent to run |
| **Runtime** | `tmux`, `process`, `ssh` | How the agent process is managed |
| **Workspace** | `worktree`, `clone` | How the working directory is created |
| **Tracker** | `github`, `linear` | Issue tracker integration |
| **SCM** | `github` | Source control (PR status, reviews, merge) |
| **Notifier** | `desktop`, `slack`, `webhook` | Where notifications are sent |
| **Terminal** | `web` | Browser-based terminal access |

## Reactions

Reactions are configurable rules that fire when a session enters a specific state. Each reaction has an action, retry count, and escalation timeout.

```yaml
reactions:
  ci-failed:
    action: send_to_agent
    message: "CI failed. Check the logs and fix the issues."
    max_retries: 2
    escalation_timeout: 120

  approved-and-green:
    action: auto_merge

  agent-exited:
    action: notify
    priority: urgent
```

Built-in reactions: `ci-failed`, `changes-requested`, `bugbot-comments`, `merge-conflicts`, `approved-and-green`, `agent-stuck`, `agent-needs-input`, `agent-exited`, `all-complete`.

## Remote Execution

Projects with `ssh_config` run agents on remote machines. Ennio creates workspaces, spawns agents, and monitors sessions over SSH, with a gRPC daemon (`ennio-node`) on the remote host for structured communication.

```yaml
projects:
  - name: remote-project
    repo: git@github.com:user/repo.git
    path: /home/user/repos/repo
    ssh_config:
      host: build-server.example.com
      port: 22
      user: deploy
      auth:
        type: key
        path: ~/.ssh/id_ed25519
      strategy: tmux
      node_config:
        enabled: true
        grpc_port: 50051
```

Manage remote nodes:

```bash
ennio node status build-server.example.com
ennio node connect remote-project
ennio node disconnect remote-project
ennio node list
```

## Architecture

Ennio is structured as a Rust workspace with 15 crates:

| Crate | Purpose |
|-------|---------|
| `ennio-cli` | CLI binary (clap) |
| `ennio-core` | Shared types, config, session model |
| `ennio-services` | Orchestration logic (session manager, lifecycle manager, event bus) |
| `ennio-plugins` | Plugin implementations |
| `ennio-db` | SQLite persistence |
| `ennio-ssh` | SSH client and remote execution strategies |
| `ennio-node` | Remote gRPC daemon binary |
| `ennio-proto` | Protobuf service definitions |
| `ennio-web` | REST API (axum) |
| `ennio-tui` | Terminal dashboard (ratatui) |
| `ennio-dashboard` | Web dashboard (dioxus WASM) |
| `ennio-nats` | NATS messaging layer |
| `ennio-ledger` | Cost tracking and budgets |
| `ennio-ml` | ML trait interfaces (anomaly detection, cost prediction) |
| `ennio-observe` | OpenTelemetry integration |

## Configuration Reference

Environment variables in config values are expanded at load time using `${VAR}` syntax.

```yaml
# Server
port: 3000                    # Web API port
terminal_port: 3001           # Terminal WebSocket port
api_token: ${ENNIO_API_TOKEN} # Bearer token for API auth
cors_origins:                 # Allowed CORS origins
  - http://localhost:3000

# Storage
database_url: sqlite:ennio.db # SQLite database path
nats_url: nats://127.0.0.1:4222

# Defaults (overridable per project)
defaults:
  runtime: tmux               # tmux | process | ssh
  agent: claude-code          # claude-code | aider | codex | opencode
  workspace: worktree         # worktree | clone
  notifiers: [desktop]

# Notifications
notifiers:
  - name: team-slack
    provider: slack
    webhook_url: ${SLACK_WEBHOOK}
  - name: ops-webhook
    provider: webhook
    url: https://hooks.example.com/ennio

notification_routing:
  ci-failed: [team-slack]
  agent-exited: [team-slack, ops-webhook]

# Projects
projects:
  - name: my-project
    repo: git@github.com:user/repo.git
    path: /home/user/repos/my-project
    default_branch: main
    max_sessions: 3
    session_prefix: mp
    tracker_config:
      provider: github         # github | linear
      owner: user
      repo: repo
      token: ${GITHUB_TOKEN}
    scm_config:
      provider: github
      owner: user
      repo: repo
      token: ${GITHUB_TOKEN}
    symlinks:
      - source: ../shared/.env
        target: .env
    post_create:
      - npm install
      - cp .env.example .env
    agent_config:
      model: opus
      max_turns: 200
    agent_rules:
      - "Write tests for all new code"
    reactions:
      ci-failed:
        action: send_to_agent
        message: "CI is red. Fix it."
        max_retries: 3
```

## License

See [LICENSE](LICENSE) for details.
