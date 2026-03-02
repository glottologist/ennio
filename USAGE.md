# Ennio Usage Guide

Ennio is an AI coding agent orchestrator. It SSHs into remote machines, launches AI agents (Claude Code, Codex, Aider, OpenCode), monitors their progress through PR lifecycle stages, and reacts to CI failures, review feedback, and merge conflicts automatically.

## Building

```bash
cargo build --workspace --release
```

The binary is `target/release/ennio`.

**Requirements**: Rust 1.88+, PostgreSQL, NATS server.

## Configuration

Ennio searches upward from the current directory for a config file:

- `ennio.yaml` / `ennio.yml`
- `.ennio.yaml` / `.ennio.yml`

Generate a starter config with `ennio init`:

```bash
ennio init                    # Creates ./ennio.yaml
ennio init /path/to/project   # Creates /path/to/project/ennio.yaml
```

The command serializes `OrchestratorConfig::default()` to YAML. It refuses to overwrite an existing `ennio.yaml` and creates parent directories as needed.

### Generated Config

Running `ennio init` produces:

```yaml
port: 3000
terminal_port: 3001
ready_threshold: 2000
defaults:
  runtime: tmux
  agent: claude-code
  workspace: worktree
projects:
- name: my-project
  repo: https://github.com/owner/repo
  path: /absolute/path/to/project
  default_branch: main
reactions:
  ci-failed:
    enabled: true
    action: send_to_agent
    priority: action
    escalate_after: 120
    retries: 2
    include_summary: false
  changes-requested:
    enabled: true
    action: send_to_agent
    priority: action
    escalate_after: 1800
    retries: 0
    include_summary: false
  # ... plus 7 more default reactions
```

Edit the `name`, `repo`, and `path` fields under `projects` to point at your actual project, then run `ennio start`.

### Full Config Reference

```yaml
port: 3000                    # Web API port
terminal_port: 3001           # Terminal WebSocket port
direct_terminal_port: 3002    # Optional direct terminal port
ready_threshold: 2000         # Milliseconds before agent is considered idle

defaults:
  runtime: tmux               # tmux | ssh | process
  agent: claude-code          # claude-code | aider | codex | opencode
  workspace: worktree         # worktree | clone
  notifiers: []               # Default notifier names

projects:
  - name: my-project
    repo: git@github.com:org/repo.git
    path: /home/user/repos/my-project
    default_branch: main
    session_prefix: mp         # Custom prefix for session IDs
    runtime: tmux              # Override default
    agent: claude-code         # Override default
    workspace: worktree        # Override default
    max_sessions: 5            # Limit concurrent sessions

    agent_config:
      permissions: full        # Adds --dangerously-skip-permissions
      model: claude-sonnet-4-20250514
      passthrough: {}          # Extra key-value args forwarded to agent

    tracker_config:
      plugin: github
      config:
        token: ghp_...

    scm_config:
      plugin: github
      config:
        token: ghp_...

    symlinks:                  # Created in each workspace
      - source: /home/user/.env
        target: .env

    post_create:               # Run after workspace creation
      - npm install
      - cargo build

    agent_rules: |             # Additional instructions for the agent
      Focus on writing tests first.
      Do not modify the database schema.

    reactions:                 # Per-project reaction overrides
      ci-failed:
        enabled: true
        action: send_to_agent
        priority: action
        retries: 3
        escalate_after: 60

notifiers:
  - name: slack-main
    plugin: slack
    config:
      webhook_url: https://hooks.slack.com/services/...
      channel: "#dev-agents"

  - name: desktop
    plugin: desktop
    config: {}

  - name: ops-webhook
    plugin: webhook
    config:
      url: https://example.com/webhook

notification_routing:
  - events: [CiFailed, AgentStuck, AgentExited]
    notifiers: [slack-main, desktop]
    priority_filter: Urgent

  - events: [AllComplete, PrMerged]
    notifiers: [slack-main]

reactions:
  ci-failed:
    enabled: true
    action: send_to_agent
    priority: action
    escalate_after: 120          # Seconds
    retries: 2
    include_summary: false
  changes-requested:
    enabled: true
    action: send_to_agent
    priority: action
    escalate_after: 1800
    retries: 0
    include_summary: false
  bugbot-comments:
    enabled: true
    action: send_to_agent
    priority: action
    escalate_after: 1800
    retries: 0
    include_summary: false
  merge-conflicts:
    enabled: true
    action: send_to_agent
    priority: action
    escalate_after: 900
    retries: 0
    include_summary: false
  approved-and-green:
    enabled: true
    action: notify
    priority: action
    retries: 0
    include_summary: false
  agent-stuck:
    enabled: true
    action: notify
    priority: urgent
    threshold: 600               # Seconds
    retries: 0
    include_summary: false
  agent-needs-input:
    enabled: true
    action: notify
    priority: urgent
    retries: 0
    include_summary: false
  agent-exited:
    enabled: true
    action: notify
    priority: urgent
    retries: 0
    include_summary: false
  all-complete:
    enabled: true
    action: notify
    priority: info
    retries: 0
    include_summary: true
```

### Data Directory

Ennio stores session data under `~/.ennio/`:

```
~/.ennio/
  {config_hash}-{project_id}/
    sessions/
    worktrees/
    archive/
```

The config hash is the first 12 hex characters of the SHA-256 of the project path.

## CLI Commands

### Orchestrator Lifecycle

```bash
ennio start                       # Start the lifecycle polling loop
ennio stop                        # Stop the orchestrator
```

### Session Management

```bash
# Spawn a new session
ennio spawn my-project
ennio spawn my-project --issue 42
ennio spawn my-project --prompt "Fix the login bug"
ennio spawn my-project --branch feature/auth --role backend
ennio spawn my-project -i 42 -p "Implement the feature" -b feature/foo -r lead

# List sessions
ennio session list
ennio session list my-project

# Session details
ennio session info <session-id>

# Kill a session (destroys runtime + workspace)
ennio session kill <session-id>

# Restore an exited session
ennio session restore <session-id>

# Send a message to a running session
ennio send <session-id> "Please also add tests"
```

### Status and Dashboard

```bash
ennio status                      # All sessions
ennio status my-project           # Filter by project

ennio dashboard                   # Open web dashboard on port 3000
ennio dashboard -p 8080           # Custom port

ennio open <session-id>           # Open session terminal in browser
ennio open all                    # Open all sessions
```

### Output Format

```bash
ennio --format table status       # Default: table output
ennio --format json session list  # JSON output
```

### Custom Config Path

```bash
ennio -c /path/to/ennio.yaml start
```

## Session Lifecycle

A session progresses through these states:

```
Spawning -> Working -> PR Open/Draft -> CI Passing/Failed -> Review Pending
  -> Approved -> Merged -> Done
```

**All 16 states:**

| State | Description |
|---|---|
| `Spawning` | Workspace and runtime being created |
| `Working` | Agent is actively coding |
| `PrOpen` | Pull request opened |
| `PrDraft` | Draft PR opened |
| `CiPassing` | CI checks passing |
| `CiFailed` | CI checks failed |
| `CiFixSent` | Agent sent a CI fix attempt |
| `CiFixFailed` | CI fix attempt also failed |
| `ReviewPending` | Waiting for code review |
| `ChangesRequested` | Reviewer requested changes |
| `Approved` | PR approved |
| `MergeConflicts` | Merge conflicts detected |
| `Merged` | PR merged (terminal) |
| `Done` | Session completed (terminal) |
| `Exited` | Agent exited unexpectedly (restorable) |
| `Killed` | Manually killed (terminal) |

**Reactions** fire automatically when certain states are reached:

| Trigger | Default Action | What Happens |
|---|---|---|
| CI failed | `send_to_agent` | Sends fix instructions to the agent (retries: 2, escalate_after: 120s) |
| Changes requested | `send_to_agent` | Sends review feedback to the agent (escalate_after: 1800s) |
| Bugbot comments | `send_to_agent` | Sends bugbot feedback to the agent (escalate_after: 1800s) |
| Merge conflicts | `send_to_agent` | Sends merge resolution instructions (escalate_after: 900s) |
| Approved + CI green | `notify` | Sends notification |
| Agent stuck | `notify` | Escalates to humans (threshold: 600s) |
| Agent needs input | `notify` | Escalates to humans |
| Agent exited | `notify` | Escalates to humans |
| All complete | `notify` | Sends summary notification (include_summary: true) |

Each reaction has configurable `retries`, `escalate_after` (seconds), and `threshold` (seconds) fields to prevent loops.

## Plugins

### Runtimes

| Plugin | Description |
|---|---|
| `tmux` | Local tmux session. Uses send-keys for short messages, load-buffer for long ones. |
| `ssh` | Remote execution via SSH. Delegates to an SSH session strategy. |
| `process` | Raw shell process (`sh -c`). No interactive messaging. |

### Agents

| Plugin | Description |
|---|---|
| `claude-code` | Claude Code CLI. Supports `--dangerously-skip-permissions`, `--model`, `--system-prompt-file`, `-p` prompt. Detects activity from terminal output and `.claude/activity.jsonl`. |
| `aider` | Aider CLI (stub). |
| `codex` | Codex CLI (stub). |
| `opencode` | OpenCode CLI (stub). |

### Workspaces

| Plugin | Description |
|---|---|
| `worktree` | Git worktree (`git worktree add`). Fast, shares object store. Path: `.ennio-worktrees/{project}/{session}`. |
| `clone` | Full git clone. Independent copy. Supports symlinks and post-create commands. Path: `.ennio-clones/{project}/{session}`. |

### Trackers

| Plugin | Description |
|---|---|
| `github` | GitHub Issues API. Fetches issues, generates branch names (`issue-{id}-{title}`), creates prompts from issue body + labels. |
| `linear` | Linear issue tracker (stub). |

### SCM

| Plugin | Description |
|---|---|
| `github` | GitHub Pull Requests API. Detects PRs by branch, checks CI status, review decisions, merge readiness. Merge method: squash. |

### Notifiers

| Plugin | Description |
|---|---|
| `desktop` | Desktop notifications via `notify-send` (Linux) or `osascript` (macOS). |
| `slack` | Slack incoming webhook. Supports channel override and action buttons. |
| `webhook` | Generic JSON POST to a configured URL. |

### Terminals

| Plugin | Description |
|---|---|
| `web` | Opens session in browser at `{base_url}/sessions/{id}`. |

## SSH Remote Execution

Ennio can run agents on remote machines via SSH using three strategies:

### Tmux Strategy

Creates a tmux session on the remote host. Best for persistent interactive sessions.

```yaml
# Agent runs inside a remote tmux session
# Messages sent via tmux send-keys / load-buffer
```

### Tmate Strategy

Uses tmate for shareable sessions with web and SSH URLs. Useful for debugging and collaboration.

### Remote Control Strategy

Starts `claude remote-control` on the remote host and communicates via its HTTP API. The orchestrator polls the remote for the session URL, then sends messages and reads output over HTTP.

### SSH Configuration

```rust
host: "remote.example.com"
port: 22
username: "deploy"
auth:
  # Key-based (recommended)
  key:
    path: /home/user/.ssh/id_ed25519
    passphrase: null              # Optional

  # Or password-based
  password:
    password: "..."

connection_timeout: 30s
keepalive_interval: 15s
host_key_policy: Strict           # Strict | AcceptNew | AcceptAll
```

Passwords and passphrases are redacted from logs and serialization.

## Web API

Base URL: `http://localhost:{port}/api/v1`

Authentication: Bearer token in `Authorization` header. Disabled when `api_token` is not set in config.

| Method | Endpoint | Description |
|---|---|---|
| GET | `/health` | Health check (no auth) |
| GET | `/sessions` | List all sessions |
| GET | `/sessions/{id}` | Get session details |
| POST | `/sessions` | Spawn a new session |
| DELETE | `/sessions/{id}` | Kill a session |
| POST | `/sessions/{id}/send` | Send message to session |

### Spawn Request

```json
POST /api/v1/sessions
{
  "project_id": "my-project",
  "issue_id": "42",
  "prompt": "Fix the login bug",
  "branch": "fix/login",
  "role": "backend"
}
```

### Send Message

```json
POST /api/v1/sessions/{id}/send
{
  "message": "Please also add integration tests"
}
```

### Error Responses

| HTTP Status | Condition |
|---|---|
| 400 | Invalid ID, bad config |
| 402 | Budget exceeded |
| 404 | Session/project not found |
| 409 | Duplicate resource |
| 504 | Timeout |
| 500 | Internal error |

All responses use `Content-Type: application/json`.

## NATS Messaging

Ennio publishes events to NATS for external integrations. Topic format:

```
ennio.{domain}.{project_id}.{action}
```

### Topic Domains

| Domain | Actions |
|---|---|
| `sessions` | spawned, restored, killed, cleaned, status_changed, activity_changed, agent_exited, agent_stuck, all_complete |
| `pr` | opened, updated, merged, closed |
| `ci` | started, passed, failed, fix_attempted |
| `review` | received, changes_requested, approved |
| `merge` | conflicts_detected, conflicts_resolved |
| `reactions` | triggered |
| `notifications` | sent |
| `metrics` | budget_warning |

### Lifecycle Topics (no project_id)

```
ennio.lifecycle.start
ennio.lifecycle.stop
ennio.commands.{command}
ennio.metrics.cost
ennio.dashboard.refresh
```

### Subscribe Patterns

```bash
# All events for a project
nats sub "ennio.*.my-project.*"

# All CI events across projects
nats sub "ennio.ci.*.*"

# All session events
nats sub "ennio.sessions.>""
```

## Dashboard (Web UI)

The Dioxus WASM dashboard provides:

- **Status bar**: total, active, attention, completed session counts
- **Attention zone**: highlights sessions needing human intervention (CI failed, changes requested, merge conflicts)
- **PR status table**: PR number, branch, CI status, review status per session
- **Session cards**: responsive grid showing each session's status, activity, branch, agent, and timestamps

Color scheme:

| Color | States |
|---|---|
| Blue | Spawning, Working |
| Purple | PR Open, PR Draft |
| Green | CI Passing, Approved |
| Red | CI Failed, CI Fix Failed |
| Amber | CI Fix Sent, Review Pending |
| Orange | Changes Requested, Merge Conflicts |
| Gray | Merged, Done |
| Dark Gray | Exited, Killed |

## TUI (Terminal UI)

Launch with any terminal that supports 256 colors:

**Keybindings:**

| Key | Action |
|---|---|
| `j` / `Down` | Next session |
| `k` / `Up` | Previous session |
| `Enter` | Toggle detail panel |
| `q` / `Esc` | Quit |

**Layout:**

- Top: session table (ID, Project, Status, Agent, Branch, Activity)
- Middle: session detail panel (toggle with Enter)
- Bottom: event log with priority-colored timestamps
- Footer: keybinding hints

## Observability

### Logging

Set the `RUST_LOG` environment variable:

```bash
RUST_LOG=info ennio start        # Default
RUST_LOG=debug ennio start       # Verbose
RUST_LOG=ennio=trace ennio start # Trace ennio crates only
```

JSON logging is available for structured log ingestion.

### Prometheus Metrics

Ennio exports the following metrics:

| Metric | Type | Description |
|---|---|---|
| `ennio_sessions_spawned_total` | Counter | Total sessions spawned |
| `ennio_sessions_killed_total` | Counter | Total sessions killed |
| `ennio_sessions_completed_total` | Counter | Total sessions completed |
| `ennio_sessions_active` | Gauge | Currently active sessions |
| `ennio_session_duration_seconds` | Histogram | Session duration |
| `ennio_events_total` | Counter | Total events emitted |
| `ennio_plugin_calls_total` | Counter | Total plugin invocations |
| `ennio_plugin_call_duration_seconds` | Histogram | Plugin call latency |
| `ennio_cost_usd_total` | Counter | Accumulated LLM cost |
| `ennio_reactions_triggered_total` | Counter | Reactions triggered |
| `ennio_reactions_escalated_total` | Counter | Reactions escalated to humans |

### OpenTelemetry

OTLP tracing export over gRPC:

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 ennio start
```

## Cost Tracking and Budgets

Ennio includes a double-entry ledger for tracking LLM costs.

### Budget Scopes

- **Global**: across all projects
- **Project**: per project
- **Session**: per session

### Budget Periods

- **Daily**: resets daily
- **Monthly**: resets monthly
- **Total**: lifetime limit

### Database Metrics

Per-session metrics tracked:

- Input/output token counts
- Estimated USD cost
- CI run count and failure count
- Review rounds
- Time to first PR (seconds)
- Time to merge (seconds)

## Database

Ennio uses PostgreSQL with 5 tables:

| Table | Purpose |
|---|---|
| `sessions` | Session state, workspace paths, runtime handles, metadata |
| `events` | Event log with type, priority, timestamps |
| `projects` | Project configuration snapshots |
| `session_metrics` | Per-session cost and performance metrics |

Indexes on `events(session_id)`, `events(event_type)`, and `sessions(project_id, status)`.

## Architecture

```
ennio-cli (binary)
  |
  +-- ennio-services (orchestration)
  |     +-- SessionManager    -> spawn, restore, kill, cleanup
  |     +-- LifecycleManager  -> poll, react, notify
  |     +-- PluginRegistry    -> register/get all plugin types
  |     +-- EventBus          -> tokio broadcast channel
  |     +-- ConfigLoader      -> YAML discovery and validation
  |     |
  |     +-- ennio-db          -> PostgreSQL repos
  |     +-- ennio-nats        -> NATS pub/sub
  |
  +-- ennio-plugins (implementations)
  |     +-- runtime/   (tmux, ssh, process)
  |     +-- agent/     (claude-code, aider, codex, opencode)
  |     +-- workspace/ (worktree, clone)
  |     +-- tracker/   (github, linear)
  |     +-- scm/       (github)
  |     +-- notifier/  (desktop, slack, webhook)
  |     +-- terminal/  (web)
  |     |
  |     +-- ennio-ssh  -> russh client + strategies (tmux, tmate, remote_control)
  |
  +-- ennio-web (HTTP API)
  +-- ennio-dashboard (Dioxus WASM)
  +-- ennio-tui (Ratatui terminal)
  +-- ennio-observe (logging, metrics, tracing)
  +-- ennio-ledger (cost tracking, budgets)
  +-- ennio-ml (prediction trait interfaces)
  +-- ennio-core (shared types, traits, errors, IDs, config, events)
```

## License

MIT OR Apache-2.0
